# Porcupine in Rust 🦀

[![ci](https://github.com/erwin-kok/porcupine-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/erwin-kok/porcupine-rs/actions/workflows/ci.yaml)
[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/github/license/erwin-kok/porcupine-rs.svg)](https://github.com/erwin-kok/porcupine-rs/blob/master/LICENSE)
[![crates](https://img.shields.io/crates/v/porcupine-rs.svg)](https://crates.io/crates/porcupine-rs)

A linearizability checker for distributed systems, reimplemented in Rust.

Based on the original [porcupine](https://github.com/anishathalye/porcupine) by Anish Athalye.

> **Note:** This is a learning project. The goal is design clarity, clean
> architecture, and a deeper understanding of both linearizability and Rust —
> not feature parity with or performance superiority over the Go original.


## What is linearizability?

Linearizability is a correctness condition for concurrent systems. It asks:

> *Can this interleaved, concurrent execution be explained as some valid
> sequential execution of the same operations?*

More precisely, a history is linearizable if there exists a total ordering of
all operations such that:

1. The ordering is consistent with the real-time order of non-overlapping operations.
2. Every operation in the ordering satisfies the system's sequential specification.

The points at which operations appear to take effect are called
**linearization points**.

<img src="./docs/linearizability.svg" width="500" height="250">

*This history is linearizable. The red lines mark the linearization points —
the moments at which each operation takes effect atomically.*

## Algorithm

The checker uses the **WGL algorithm** (Wing & Gong, extended by Lowe) — a
depth-first backtracking search over the space of possible linearization
orderings.

At each step the search:

1. Scans the active history for an **eligible** operation.
2. Asks the model whether that operation is **accepted** from the current state.
3. If accepted, **lifts** (removes) it from the history, updates the model
   state, and recurses.
4. If the recursion fails, **backtracks**: restores the operation to the history
   and the model to its previous state, then tries the next candidate.

A **complete linearization** is found when the active history is empty —
every operation has been successfully placed in sequential order.

### Skip-list

Naïvely, lifting an operation from the history requires copying the remaining
slice — O(n) work per level of recursion for a search tree of depth n.

Instead, the history is maintained as an **index-based doubly-linked list**
(the skip-list). Lifting an operation removes its call and return entries
in O(1); restoring them on backtrack is also O(1). 

### Cache

The same `(set of remaining operations, model state)` pair can be reached via
many different orderings of earlier operations. Any linearization reachable
from this pair once is reachable the same way every time, so re-exploring it
is wasted work.

### Partitioning

Many models decompose naturally into independent sub-problems. A key-value
store, for example, has completely independent state per key: operations on
key `A` can never affect the linearizability of operations on key `B`.

The `Model` trait exposes a `partition_operations` method that splits a
history into independent sub-histories. Each sub-history is checked in
isolation, with its own search state and cache. 


# Installation

```bash
# Clone the repository
git clone https://github.com/erwin-kok/porcupine-rs
cd porcupine-rs

# Build
cargo build --release

# Run tests
cargo test
```

## Usage

### Define a model

A model defines the sequential specification of your system. Implement the
`Model` trait:

```rust
use porcupine::{Model, Operation};

#[derive(Clone, Debug)]
pub enum RegisterOp {
    Put(u32),
    Get(Option<u32>),  // carries the value the client observed
}

#[derive(Clone, Debug)]
pub struct RegisterModel;

impl Model for RegisterModel {
    type State    = u32;          // current value of the register
    type Op       = RegisterOp;
    type Metadata = ();

    fn init() -> u32 { 0 }

    fn step(state: &u32, op: &RegisterOp) -> (bool, u32) {
        match op {
            // A Put always succeeds and updates the state.
            RegisterOp::Put(v) => (true, *v),
            // A Get succeeds only if the observed value matches the current state.
            RegisterOp::Get(v) => (*v == Some(*state), *state),
        }
    }
}
```

### Check an operation history

An `Operation` records what happened (`op`), when it was invoked (`call_time`), and when it returned (`return_time`):


```rust
    fn put(client_id: u32, call: i64, ret: i64, v: u32) -> Operation<RegisterModel> {
        Operation {
            client_id: Some(client_id),
            call_time: call,
            return_time: ret,
            op: RegisterOp::Put(v),
            metadata: None,
        }
    }

    fn get(client_id: u32, call: i64, ret: i64, v: Option<u32>) -> Operation<RegisterModel> {
        Operation {
            client_id: Some(client_id),
            call_time: call,
            return_time: ret,
            op: RegisterOp::Get(v),
            metadata: None,
        }
    }

    let history = vec![
        put(0,  10, 100, 1),
        get(2,  80, 210, Some(2)),
        get(3, 110, 230, Some(1)),
        put(1, 120, 210, 2),
    ];

    assert!(porcupine::check_operations(&history));
```

### Check an event history

If you record call and return events separately (as produced by many testing frameworks), use the `EventModel` trait and `check_events`:

```rust
use porcupine::{Event, EventModel};

#[derive(Clone, Debug)]
pub enum RegisterInput { 
    Put(u32), 
    Get 
}

#[derive(Clone, Debug)]
pub enum RegisterOutput { 
    Put,      
    Get(Option<u32>) 
}

impl EventModel for RegisterModel {
    type Input  = RegisterInput;
    type Output = RegisterOutput;

    fn combine(input: &RegisterInput, output: &RegisterOutput) -> RegisterOp {
        match (input, output) {
            (RegisterInput::Put(v), RegisterOutput::Put)       => RegisterOp::Put(*v),
            (RegisterInput::Get,    RegisterOutput::Get(v))    => RegisterOp::Get(*v),
            _ => panic!("mismatched input/output"),
        }
    }
}

let history = vec![
    Event::Call   { client_id: Some(0), value: RegisterInput::Put(1), id: 0, metadata: None },
    Event::Return { client_id: Some(0), value: RegisterOutput::Put,   id: 0, metadata: None },
    // ...
];

assert!(porcupine::check_events(&history));
```

## Limitations

- **No visualization.** The Go original can produce an HTML visualization of
  the history and linearization. This is not implemented.
- **No `NondeterministicModel`.** The Go original supports models whose `step`
  returns multiple possible next states. Not yet implemented.
- **Timeout is approximate.** The kill flag is checked at iteration boundaries,
  not at arbitrary points, so the actual wall time may slightly exceed the
  requested timeout.
- **Single binary.** There is no CLI; the library is used programmatically.


# Inspiration

This project is based on the excellent work by Anish Athalye:

Porcupine (Go): https://github.com/anishathalye/porcupine
