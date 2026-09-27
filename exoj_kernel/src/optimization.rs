//! optimization.rs — spreadsheet instance-logic coordinate steppers.
//!
//! Deterministic, gradient-free optimization over the packed 2-bit matrix.
//! No backprop, no FP gradients: each pass walks the gate's cells in a fixed
//! order, flashes the three switch configurations for the current cell
//! (Blocked → Muted → Positive), scores the whole stored log, and permanently
//! commits whichever state minimizes total tracking error. Ties keep the
//! current state (hysteresis against oscillation).
//!
//! Error metric: sum over log rows of |score(row) − target(row)| (integers
//! only; scores come from exoj_kernel's integer evaluation loop).

use crate::ternary::{PackedTernary, STATE_BLOCKED, STATE_MUTED, STATE_POSITIVE};
use quilt_storage::fabric::Fabric;

pub const CANDIDATE_STATES: [i8; 3] = [STATE_BLOCKED, STATE_MUTED, STATE_POSITIVE];

/// The tracking objective: targets[i] is the desired score for fabric row i.
pub struct Objective<'a> {
    pub fabric: &'a Fabric,
    pub targets: &'a [i64],
}

impl<'a> Objective<'a> {
    pub fn total_error(&self, gate: &PackedTernary) -> i64 {
        let mut err: i64 = 0;
        for row in 0..self.fabric.rows() {
            let s = gate.score_row(self.fabric.row(row));
            err += (s - self.targets[row as usize]).abs();
        }
        err
    }
}

/// One coordinate-descent pass. Returns (total_error_after, cells_changed).
/// Deterministic: cell order is 0..len, candidate order is fixed.
pub fn run_pass(
    gate: &mut PackedTernary,
    objective: &Objective,
    max_rows: Option<u64>,
) -> (i64, usize) {
    let rows = max_rows.unwrap_or(objective.fabric.rows()).min(objective.fabric.rows());
    let mut changed = 0usize;

    for cell in 0..gate.len() {
        let original = gate.get(cell);
        let mut best_state = original;
        let mut best_err = i64::MAX;

        for &state in &CANDIDATE_STATES {
            gate.set(cell, state);
            let mut err: i64 = 0;
            for row in 0..rows {
                let s = gate.score_row(objective.fabric.row(row));
                err += (s - objective.targets[row as usize]).abs();
            }
            if err < best_err {
                best_err = err;
                best_state = state;
            }
        }
        gate.set(cell, best_state);
        if best_state != original {
            changed += 1;
        }
    }

    let total = objective.total_error(gate);
    (total, changed)
}

/// Run passes until the error stops improving or `max_passes` is hit.
/// Returns the per-pass error trace.
pub fn run_to_convergence(
    gate: &mut PackedTernary,
    objective: &Objective,
    max_passes: usize,
) -> Vec<i64> {
    let mut trace = Vec::with_capacity(max_passes);
    let mut prev = objective.total_error(gate);
    trace.push(prev);
    for _ in 0..max_passes {
        let (err, _changed) = run_pass(gate, objective, None);
        trace.push(err);
        if err >= prev {
            break; // no improvement — converged (or stuck; determinism holds)
        }
        prev = err;
    }
    trace
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fixture(dir: &std::path::Path) -> (Fabric, Vec<i64>) {
        // two well-separated clusters of 3 rows, 8 dims
        let mut f = Fabric::create(dir.join("fixture.fab"), 8).unwrap();
        let mut targets = Vec::new();
        for i in 0..3 {
            let mut v = vec![0.0f32; 8];
            v[i] = 16.0; // sparse "positive" rows
            f.append(&v).unwrap();
            targets.push(64);
        }
        for i in 0..3 {
            let mut v = vec![0.0f32; 8];
            v[i] = -16.0; // sparse "negative" rows
            f.append(&v).unwrap();
            targets.push(-64);
        }
        (f, targets)
    }

    #[test]
    fn stepper_reduces_error_and_is_deterministic() {
        let dir = std::env::temp_dir().join(format!(
            "eos-logic-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        let (fabric, targets) = make_fixture(&dir);
        let objective = Objective { fabric: &fabric, targets: &targets };

        let mut a = PackedTernary::seeded(8, 42);
        let trace_a = run_to_convergence(&mut a, &objective, 10);
        assert!(*trace_a.last().unwrap() < trace_a[0], "error must decrease");

        // determinism: same seed → same trace
        let mut b = PackedTernary::seeded(8, 42);
        let trace_b = run_to_convergence(&mut b, &objective, 10);
        assert_eq!(trace_a, trace_b);

        // converged gate classifies the clusters correctly
        for row in 0..3 {
            assert!(a.score_row(fabric.row(row)) > 0);
        }
        for row in 3..6 {
            assert!(a.score_row(fabric.row(row)) < 0);
        }
    }
}
