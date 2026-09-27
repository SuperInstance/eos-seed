//! eos-seed — the bootstrap seed of the Epigenetic Operating System.
//!
//! A short, click-and-play demonstration of the full loop on simulated log
//! vectors: quilt_storage (memory-mapped fabric) ← exoj_kernel (2-bit packed
//! ternary gating, integer-only scoring) ← instance_logic (coordinate
//! stepper). Deterministic seed → identical output every run.

use exoj_kernel::PackedTernary;
use instance_logic::{run_pass, Objective, run_to_convergence};
use quilt_storage::Fabric;

/// Deterministic xorshift64* — the only randomness in the seed.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn unit(&mut self) -> f32 {
        ((self.next() >> 40) as f32) / (1u64 << 24) as f32 * 2.0 - 1.0
    }
}

fn main() {
    const DIMS: usize = 32;
    const ROWS: usize = 96;
    const CLASSES: usize = 3;
    const ROWS_PER_CLASS: usize = ROWS / CLASSES;
    const SEED: u64 = 2718;

    let tmp = std::env::temp_dir().join("eos-seed-demo.fab");
    let _ = std::fs::remove_file(&tmp);

    // ---- 1. fabric: simulated sonar log, class-signature rows -------------
    let mut fabric = Fabric::create(&tmp, DIMS as u64)
        .expect("create fabric");
    let mut rng = Lcg(SEED);
    let mut targets = Vec::with_capacity(ROWS);
    for row in 0..ROWS {
        let class = row / ROWS_PER_CLASS; // 0, 1, 2
        let mut v = [0.0f32; DIMS];
        for c in v.iter_mut() {
            *c = rng.unit() * 0.5; // background noise
        }
        // class signature: 4 hot dims per class, sign encodes class polarity
        let base = class * 8;
        let sign = if class == 2 { -8.0 } else { 8.0 };
        for k in 0..4 {
            v[base + k] = sign + rng.unit() * 0.25;
        }
        fabric.append(&v).expect("append row");
        // target scores on the quantizer's native scale: quantize() shifts
        // to a 7-bit fixed point (≈65 per unit-signature dim), so the ideal
        // gate (4 hot dims per class, sign-split) yields ±~256. Class 2 is
        // the blocked/negative class.
        targets.push(if class == 2 { -256 } else { 256 });
    }
    println!("fabric: {ROWS} rows × {DIMS} dims appended at {}", tmp.display());

    // ---- 2. gate: deterministic seeded 2-bit ternary ----------------------
    let mut gate = PackedTernary::seeded(DIMS, SEED);
    let objective = Objective { fabric: &fabric, targets: &targets };
    let e0 = objective.total_error(&gate);
    println!("initial gate error: {e0}");

    // ---- 3. stepper: flash-and-commit coordinate descent ------------------
    let trace = run_to_convergence(&mut gate, &objective, 8);
    println!("pass trace (total |score-target|):");
    for (i, e) in trace.iter().enumerate() {
        println!("  pass {i}: error {e}");
    }

    // per-class mean score after evolution
    println!("per-class mean scores after evolution:");
    for class in 0..CLASSES {
        let lo = (class * ROWS_PER_CLASS) as u64;
        let hi = lo + ROWS_PER_CLASS as u64;
        let mean: f64 = (lo..hi)
            .map(|r| gate.score_row(fabric.row(r)) as f64)
            .sum::<f64>() / ROWS_PER_CLASS as f64;
        println!("  class {class}: {mean:.1}");
    }

    // final single-pass audit: confirm convergence is stable
    let (err, changed) = run_pass(&mut gate, &objective, None);
    println!("audit pass: error {err}, {changed} cells changed");
    println!("packed gate: {} cells in {} bytes (2 bits each)",
             gate.len(), gate.as_bytes().len());
    println!("done — the seed lives.");
}
