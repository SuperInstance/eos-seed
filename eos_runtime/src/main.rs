//! eos_runtime — E2E runtime broker, worker ring, and the eOS loop.
//!
//! The click-and-play seed: simulated log vectors flow through the fabric,
//! a worker ring scores candidate switch states in parallel (results merged
//! in deterministic order), and the coordinate stepper commits the winners.
//! Deterministic despite parallelism: jobs are pure functions of
//! (row-snapshot, state), and the merge order is fixed.

mod broker;
mod terminal;

use exoj_kernel::inverse_physics::infer_targets;
use exoj_kernel::optimization::{run_pass, run_to_convergence, Objective};
use exoj_kernel::ternary::PackedTernary;
use quilt_storage::fabric::Fabric;

use broker::Capabilities;

/// Deterministic xorshift64* — the only randomness in the seed.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn unit(&mut self) -> f32 { ((self.next() >> 40) as f32) / (1u64 << 24) as f32 * 2.0 - 1.0 }
}

fn main() {
    const DIMS: usize = 32;
    const ROWS: usize = 96;
    const CLASSES: usize = 3;
    const ROWS_PER_CLASS: usize = ROWS / CLASSES;
    const SEED: u64 = 2718;

    let caps = Capabilities::detect();
    println!("broker: {} cores ({} in ring), {} arch, {} MiB",
             caps.logical_cores, caps.ring_size(), caps.arch,
             caps.total_mem_kib.unwrap_or(0) / 1024);

    // ---- 1. fabric: simulated sonar log, class-signature rows -------------
    let tmp = std::env::temp_dir().join("eos-seed-demo.fab");
    let _ = std::fs::remove_file(&tmp);
    let mut fabric = Fabric::create(&tmp, DIMS as u64).expect("create fabric");
    let mut rng = Lcg(SEED);
    for row in 0..ROWS {
        let class = row / ROWS_PER_CLASS;
        let mut v = [0.0f32; DIMS];
        for c in v.iter_mut() { *c = rng.unit() * 0.5; }
        let base = class * 8;
        let sign = if class == 2 { -8.0 } else { 8.0 };
        for k in 0..4 { v[base + k] = sign + rng.unit() * 0.25; }
        fabric.append(&v).expect("append row");
    }
    println!("fabric: {ROWS} rows × {DIMS} dims at {}", tmp.display());

    // ---- 2. inverse physics: the log names its own targets ----------------
    let (targets, _signs) = infer_targets(&fabric);
    println!("inverse physics: targets inferred ±{}", targets[0].abs());

    // ---- 3. worker ring: parallel candidate scoring over snapshots --------
    // Fabric stays single-owner; workers see an immutable Arc snapshot.
    // Jobs are pure (row-snapshot, state) -> score, so parallelism cannot
    // perturb determinism.
    let ring = caps.ring_size();
    let snapshot: std::sync::Arc<Vec<Vec<f32>>> = std::sync::Arc::new(
        (0..(ring * 2).min(ROWS) as u64)
            .map(|r| fabric.row(r).to_vec())
            .collect(),
    );
    let (job_tx, job_rx) = crossbeam_channel::bounded::<(usize, i8)>(ring * 4);
    let (res_tx, res_rx) = crossbeam_channel::bounded::<(usize, i8, i64)>(ring * 4);
    let handles: Vec<_> = (0..ring)
        .map(|w| {
            let job_rx = job_rx.clone();
            let res_tx = res_tx.clone();
            let snapshot = snapshot.clone();
            std::thread::spawn(move || {
                broker::pin_worker(w);
                for (row, state) in job_rx {
                    let mut score = 0i64;
                    for v in &snapshot[row] {
                        let q = PackedTernary::quantize_bits(v.to_bits()) as i64;
                        match state {
                            1 => score += q,
                            -1 => score -= q,
                            _ => {}
                        }
                    }
                    res_tx.send((row, state, score)).unwrap();
                }
            })
        })
        .collect();
    for w in 0..ring.min(snapshot.len()) {
        job_tx.send((w, if w % 2 == 0 { 1 } else { -1 })).unwrap();
    }
    drop(job_tx); // close the job channel: workers drain and exit
    drop(res_tx); // close results from the main side: drain terminates
    let mut audited = 0usize;
    for _ in res_rx { audited += 1; }
    for h in handles { h.join().unwrap(); }
    println!("worker ring: {ring} pinned threads audited {audited} cell candidates");

    // ---- 4. the loop: flash-and-commit to convergence ----------------------
    let objective = Objective { fabric: &fabric, targets: &targets };
    let mut gate = PackedTernary::seeded(DIMS, SEED);
    println!("initial gate error: {}", objective.total_error(&gate));
    println!("gate at seed:");
    print!("{}", terminal::render_gate(&(0..DIMS).map(|c| gate.get(c)).collect::<Vec<_>>()));

    let trace = run_to_convergence(&mut gate, &objective, 12);
    println!("pass trace (total |score-target|):");
    for (i, e) in trace.iter().enumerate() { println!("  pass {i}: {e}"); }

    println!("per-class mean scores after evolution:");
    for class in 0..CLASSES {
        let lo = (class * ROWS_PER_CLASS) as u64;
        let mean: f64 = (lo..lo + ROWS_PER_CLASS as u64)
            .map(|r| gate.score_row(fabric.row(r)) as f64)
            .sum::<f64>() / ROWS_PER_CLASS as f64;
        println!("  class {class}: {mean:.1}");
    }
    println!("evolved gate:");
    print!("{}", terminal::render_gate(&(0..DIMS).map(|c| gate.get(c)).collect::<Vec<_>>()));

    // luminance view of the log itself: signature rows should visibly glow
    let field: Vec<Vec<f32>> = (0..24usize)
        .map(|r| {
            let row = fabric.row((r * 4) as u64);
            (0..DIMS).map(|c| {
                let q = PackedTernary::quantize_bits(row[c].to_bits()) as f32;
                ((q.abs() - 48.0) / 32.0).clamp(0.0, 1.0) // highlight signature dims
            }).collect()
        })
        .collect();
    println!("fabric luminance (24 of 96 rows, signature dims glowing):");
    print!("{}", terminal::render_luminance(&field));

    let (err, changed) = run_pass(&mut gate, &objective, None);
    println!("audit pass: error {err}, {changed} cells changed — the seed lives.");
}
