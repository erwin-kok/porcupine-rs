use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;

use serde::Serialize;

use crate::linearization_info::{Annotation, LinearizationInfo};
use crate::model::Model;

// ---------------------------------------------------------------------------
// Embedded static assets
// ---------------------------------------------------------------------------

const HTML_TEMPLATE: &str = include_str!("../visualization/index.html");
const CSS: &str = include_str!("../visualization/index.css");
const JS: &str = include_str!("../visualization/index.js");

const PLACEHOLDER_CSS: &str = "/*{CSS}*/";
const PLACEHOLDER_JS: &str = "/*{JS}*/";
const PLACEHOLDER_DATA: &str = "/*{DATA}*/";

// ---------------------------------------------------------------------------
// JSON-serializable DTOs
// ---------------------------------------------------------------------------
//
// Field names use camelCase to match what index.js expects.

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct HistoryElement {
    pub client_id: u32,
    pub start: i64,
    pub original_start: String,
    pub end: i64,
    pub original_end: String,
    pub description: String,
    pub metadata: String,
    pub annotation: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct LinearizationStep {
    pub index: usize,
    pub state_description: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct PartitionVisualizationData {
    pub history: Vec<HistoryElement>,
    pub partial_linearizations: Vec<Vec<LinearizationStep>>,
    pub largest: BTreeMap<usize, usize>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationElement {
    pub client_id: Option<u32>,
    pub tag: Option<String>,
    pub start: i64,
    pub end: i64,
    pub description: String,
    pub details: String,
    pub annotation: bool,
    pub text_color: String,
    pub background_color: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct VisualizationData {
    pub partitions: Vec<PartitionVisualizationData>,
    pub annotations: Vec<AnnotationElement>,
}

// ---------------------------------------------------------------------------
// Timestamp mapping
// ---------------------------------------------------------------------------

/// Map every raw timestamp in the info to an integer spaced at least 100 apart.
///
/// This ensures that the JS epsilon-adjustment (epsilon = 16) can nudge
/// linearization point markers without them crossing neighbouring timestamps.
fn timestamp_mapping<M: Model>(info: &LinearizationInfo<M>) -> BTreeMap<i64, i64> {
    // Collect all distinct timestamps.
    let mut all: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();

    for ops in &info.partition_ops {
        for op in ops {
            all.insert(op.call_time);
            all.insert(op.return_time);
        }
    }
    for ann in &info.annotations {
        all.insert(ann.start);
        if let Some(end) = ann.end {
            all.insert(end);
        }
    }

    // Map i-th distinct timestamp → i * 100.
    all.into_iter()
        .enumerate()
        .map(|(i, ts)| (ts, i as i64 * 100))
        .collect()
}

// ---------------------------------------------------------------------------
// Core data builder
// ---------------------------------------------------------------------------

pub fn compute_visualization_data<M: Model>(info: &LinearizationInfo<M>) -> VisualizationData {
    let time_map = timestamp_mapping(info);

    let partitions: Vec<PartitionVisualizationData> = info
        .partition_ops
        .iter()
        .zip(info.partial_linearizations.iter())
        .map(|(ops, seqs)| build_partition_data::<M>(ops, seqs, &time_map))
        .collect();

    let annotations: Vec<AnnotationElement> = info
        .annotations
        .iter()
        .map(|a| build_annotation(a, &time_map))
        .collect();

    VisualizationData {
        partitions,
        annotations,
    }
}

fn build_partition_data<M: Model>(
    ops: &[crate::model::Operation<M>],
    seqs: &[Vec<usize>],
    time_map: &BTreeMap<i64, i64>,
) -> PartitionVisualizationData {
    // -----------------------------------------------------------------------
    // Build the history elements (one per operation).
    // -----------------------------------------------------------------------
    let history: Vec<HistoryElement> = ops
        .iter()
        .map(|op| HistoryElement {
            client_id: op.client_id.unwrap_or(0),
            start: time_map[&op.call_time],
            original_start: op.call_time.to_string(),
            end: time_map[&op.return_time],
            original_end: op.return_time.to_string(),
            description: M::describe_operation(&op.op),
            metadata: M::describe_metadata(op.metadata.as_ref()),
            annotation: false,
        })
        .collect();

    let mut sorted_seqs: Vec<&Vec<usize>> = seqs.iter().collect();
    sorted_seqs.sort_by_key(|b| std::cmp::Reverse(b.len()));

    // `largest[op_index]` = index (into sorted_seqs) of the longest
    // partial linearization that contains that operation.
    let mut largest: BTreeMap<usize, usize> = BTreeMap::new();
    let mut largest_size: BTreeMap<usize, usize> = BTreeMap::new();

    let linearizations: Vec<Vec<LinearizationStep>> = sorted_seqs
        .iter()
        .enumerate()
        .map(|(lin_idx, seq)| {
            let mut state = M::init();
            seq.iter()
                .map(|&op_idx| {
                    let op = &ops[op_idx];
                    let (ok, next_state) = M::step(&state, &op.op);
                    if !ok {
                        panic!(
                            "valid partial linearization returned non-ok from step \
                             (op_index={op_idx})"
                        );
                    }
                    state = next_state;
                    let state_desc = M::describe_state(&state);

                    // Track largest: prefer the linearization sequence whose
                    // length is greatest for this op_index.
                    let current_largest = largest_size.get(&op_idx).copied().unwrap_or(0);
                    if seq.len() > current_largest {
                        largest_size.insert(op_idx, seq.len());
                        largest.insert(op_idx, lin_idx);
                    }

                    LinearizationStep {
                        index: op_idx,
                        state_description: state_desc,
                    }
                })
                .collect()
        })
        .collect();

    PartitionVisualizationData {
        history,
        partial_linearizations: linearizations,
        largest,
    }
}

fn build_annotation(a: &Annotation, time_map: &BTreeMap<i64, i64>) -> AnnotationElement {
    let end = a.end.unwrap_or(a.start); // already normalised in add_annotations
    AnnotationElement {
        client_id: a.client_id,
        tag: a.tag.clone(),
        start: time_map[&a.start],
        end: time_map[&end],
        description: a.description.clone(),
        details: a.details.clone(),
        annotation: true,
        text_color: a.text_color.clone(),
        background_color: a.background_color.clone(),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write an HTML visualization of `info` to `output`.
///
/// The resulting file can be opened directly in any web browser.
///
/// The type parameter `M` is required so the function can call
/// `M::describe_operation`, `M::describe_state`, `M::describe_metadata`, and
/// replay `M::step` over partial linearizations to annotate each step with
/// the resulting state.
pub fn visualize<M: Model, W: Write>(
    info: &LinearizationInfo<M>,
    output: &mut W,
) -> io::Result<()> {
    let data = compute_visualization_data::<M>(info);
    let json = serde_json::to_string(&data).map_err(io::Error::other)?;

    let html = HTML_TEMPLATE
        .replace(PLACEHOLDER_CSS, CSS)
        .replace(PLACEHOLDER_JS, JS)
        .replace(PLACEHOLDER_DATA, &json);

    output.write_all(html.as_bytes())
}

/// Write an HTML visualization of `info` to the file at `path`.
///
/// This is a convenience wrapper around [`visualize`].
pub fn visualize_path<M: Model>(info: &LinearizationInfo<M>, path: &Path) -> io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    visualize::<M, _>(info, &mut f)
}
