use std::{collections::HashSet, sync::Arc};

use crate::model::{Model, Operation};

// ---------------------------------------------------------------------------
// Annotation
// ---------------------------------------------------------------------------

/// An extra marker to render in the visualization alongside client operations.
///
/// Supply either `client_id` (to annotate an existing client's timeline) or
/// `tag` (to create a named independent timeline, e.g. `"server"` or
/// `"framework"`).
///
/// `end` is optional; if omitted (left as `None`) the annotation is treated as
/// a point-in-time event at `start`.
///
/// `text_color` and `background_color` are optional CSS color strings,
/// e.g. `"#efaefc"`.
#[derive(Debug, Clone, Default)]
pub struct Annotation {
    pub client_id: Option<u32>,
    pub tag: Option<String>,
    pub start: i64,
    pub end: Option<i64>,
    pub description: String,
    pub details: String,
    pub text_color: String,
    pub background_color: String,
}

// ---------------------------------------------------------------------------
// LinearizationInfo
// ---------------------------------------------------------------------------

/// Diagnostic information about a linearizability check.
///
/// Returned by [`check_operations_info`] and [`check_events_info`].
///
/// [`check_operations_info`]: crate::check_operations_info
/// [`check_events_info`]: crate::check_events_info
#[derive(Debug, Clone)]
pub struct LinearizationInfo<M: Model> {
    /// One entry per partition.  Each entry is a deduplicated list of partial
    /// (or complete) linearization sequences.
    ///
    /// A sequence is a `Vec<usize>` of `op_index` values in linearization order.
    pub partial_linearizations: Vec<Vec<Vec<usize>>>,

    /// The original operations for each partition, in `op_index` order.
    ///
    /// `partition_ops[p][i]` is the `Operation<M>` for `op_index = i` in
    /// partition `p`.  Used by the visualizer to read timestamps, descriptions,
    /// and metadata.
    pub partition_ops: Vec<Vec<Operation<M>>>,

    /// Optional extra annotations added by the caller.
    pub annotations: Vec<Annotation>,
}

impl<M: Model> Default for LinearizationInfo<M> {
    fn default() -> Self {
        Self {
            partial_linearizations: Vec::new(),
            partition_ops: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl<M: Model> LinearizationInfo<M> {
    /// An empty info object, used when `compute_info` is `false`.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build a `LinearizationInfo` from the raw data collected during the search.
    ///
    /// - `all_longest[p][i]` is `Some(arc)` when the search found a partial
    ///   linearization of length `arc.len()` that includes `ops[i]` of partition `p`.
    ///   Multiple indices may share the same `Arc`; we deduplicate by pointer identity.
    /// - `partition_ops[p]` is the `Operation<M>` slice for partition `p`.
    pub fn from_longest(
        all_longest: Vec<Vec<Option<Arc<Vec<usize>>>>>,
        partition_ops: Vec<Vec<Operation<M>>>,
    ) -> Self {
        let partial_linearizations = all_longest.into_iter().map(dedup_sequences).collect();

        Self {
            partial_linearizations,
            partition_ops,
            annotations: Vec::new(),
        }
    }

    /// Add extra annotations that will be rendered in the visualization
    /// alongside the client operations.
    ///
    /// See [`Annotation`] for the available fields.
    pub fn add_annotations(&mut self, annotations: Vec<Annotation>) {
        for mut a in annotations {
            // Clamp: if end is before start, treat as a point-in-time event.
            if let Some(end) = a.end {
                if end < a.start {
                    a.end = Some(a.start);
                }
            } else {
                a.end = Some(a.start);
            }
            self.annotations.push(a);
        }
    }
}

// ---------------------------------------------------------------------------
// Deduplication helper
// ---------------------------------------------------------------------------

/// Deduplicate a `longest` array by `Arc` pointer identity, then clone each
/// unique sequence into an owned `Vec<usize>`.
fn dedup_sequences(longest: Vec<Option<Arc<Vec<usize>>>>) -> Vec<Vec<usize>> {
    let mut seen: HashSet<*const Vec<usize>> = HashSet::new();
    let mut unique: Vec<Vec<usize>> = Vec::new();

    for arc in longest.iter().flatten() {
        let ptr = Arc::as_ptr(arc);
        if seen.insert(ptr) {
            unique.push(arc.as_ref().clone());
        }
    }

    unique
}
