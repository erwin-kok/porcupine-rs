use crate::model::{CheckResult, Model, Operation};

pub fn check_operations<M: Model>(history: &[Operation<M>]) -> CheckResult {
    for partition in M::partition(history) {
        if !linearizable::<M>(&partition, &M::init()) {
            return CheckResult::Illegal;
        }
    }
    CheckResult::Ok
}

fn linearizable<M: Model>(remaining: &[Operation<M>], state: &M::State) -> bool {
    if remaining.is_empty() {
        return true;
    }

    for i in 0..remaining.len() {
        if !eligible(remaining, i) {
            continue;
        }
        let op = &remaining[i];
        let (accepted, next_state) = M::step(state, &op.input, &op.output);
        if accepted {
            let rest: Vec<Operation<M>> = remaining
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, o)| o.clone())
                .collect();
            if linearizable::<M>(&rest, &next_state) {
                return true;
            }
        }
    }

    false
}

fn eligible<M: Model>(ops: &[Operation<M>], i: usize) -> bool {
    let call_start = ops[i].call;
    ops.iter()
        .enumerate()
        .all(|(j, other)| j == i || other.return_time > call_start)
}
