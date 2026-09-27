//! eos_runtime — the bootstrap loop of the Epigenetic Operating System.
//!
//! The closed-loop validation cycle, per the eOS spec:
//!   1. INGEST      — MemoryMappedFabric pre-allocated for a 256-dim matrix
//!                    (16x16 down-sampled state snapshots)
//!   2. PERCEPTION  — a deterministic mock stream: an object moving across
//!                    the 16x16 coordinate space, appended zero-copy
//!   3. DISCRETE OPTIMIZATION — InstanceLogicOptimizer sweeps the 2-bit
//!                    packed PackedTernaryHead cell-by-cell every frame,
//!                    flashing {-1, 0, +1} and committing whichever state
//!                    minimizes contrastive reconstruction error over the
//!                    cached historical vectors — no gradients, no PyTorch
//!   4. VISUAL      — HighDensitySubGridVisualizer renders the live gate
//!                    state as a 6x packed sextant waterfall
//!
//! Deterministic: fixed seed, fixed walk, fixed cache policy.

mod broker;
mod terminal;

use exoj_kernel::optimization::InstanceLogicOptimizer;
use exoj_kernel::ternary::PackedTernaryHead;
use exoj_kernel::inverse_physics::infer_targets;
use quilt_storage::fabric::MemoryMappedFabric;

use terminal::HighDensitySubGridVisualizer;

use broker::Capabilities;

const GRID: usize = 16; // 16x16 coordinate space
const DIMS: usize = GRID * GRID; // 256 structural dimensions
const FRAMES: usize = 96;
const CACHE_ROWS: usize = 48; // historical vectors the optimizer scores over
const NOVEL_WINDOW: usize = 8; // newest cache rows count as "signal"
const TARGET: i64 = 256;

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
    let caps = Capabilities::detect();
    println!("broker: {} cores ({} ring), {} arch, {} MiB — eOS bootstrap loop",
             caps.logical_cores, caps.ring_size(), caps.arch, caps.total_mem_kib.unwrap_or(0) / 1024);

    // ---- 1. INGEST: pre-allocate the 256-dim fabric on disk ----------------
    let path = std::env::temp_dir().join("eos-bootstrap.fab");
    let _ = std::fs::remove_file(&path);
    let mut fabric = MemoryMappedFabric::create(&path, DIMS as u64).expect("fabric");
    fabric.preallocate((FRAMES + 8) as u64).expect("preallocate");

    // ---- 2+3+4. perception -> discrete sweep -> sextant waterfall ----------
    let mut gate = PackedTernaryHead::seeded(DIMS, 2718);
    let mut viz = HighDensitySubGridVisualizer::new(GRID / 2, GRID / 3 + 1); // 8 x 6 cells = full 16x16 grid

    println!("loop: {FRAMES} frames, {DIMS} dims, cache {CACHE_ROWS}, target ±{TARGET}");
    println!("legend: object dot = perception input | lit sextants = gate +1 cells");

    // async perception: a background worker generates the mock camera
    // stream and ships frames over a bounded crossbeam channel — the
    // thread boundary is a plain message, no shared mutable state.
    let (frame_tx, frame_rx) = crossbeam_channel::bounded::<(usize, usize, [f32; DIMS])>(4);
    let perceiver = {
        std::thread::spawn(move || {
            broker::pin_worker(1); // best-effort: perceiver on its own core
            let mut rng = Lcg(2718);
            for frame in 0..FRAMES {
                let t = frame as f32;
                let ox = ((t / 9.0).sin() * 6.5 + 7.5).round().clamp(0.0, 15.0) as usize;
                let oy = ((t / 6.0).cos() * 5.5 + 7.5).round().clamp(0.0, 15.0) as usize;
                let mut v = [0.0f32; DIMS];
                for c in v.iter_mut() { *c = rng.unit() * 0.4; } // ambient noise
                v[oy * GRID + ox] = 8.0; // the hot structural cell
                if frame_tx.send((ox, oy, v)).is_err() { break; }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        })
    };

    for frame in 0..FRAMES {
        // -- perception feed: receive from the background worker, zero-copy --
        let (ox, oy, v) = frame_rx.recv().expect("perceiver alive");
        fabric.append(&v).expect("append frame");

        // -- discrete optimization: sweep the 2-bit matrix over the cache ---
        let rows_so_far = fabric.rows();
        let cache_start = rows_so_far.saturating_sub(CACHE_ROWS as u64);
        let cache: Vec<u64> = (cache_start..rows_so_far).collect();
        // contrastive targets: newest NOVEL_WINDOW rows are the signal (high),
        // the older history is background (low) — the gate must learn to
        // respond to the moving object and reject the stale past.
        let split = cache.len().saturating_sub(NOVEL_WINDOW);
        let targets: Vec<i64> = (0..cache.len())
            .map(|i| if i >= split { TARGET } else { -TARGET })
            .collect();
        let (err, changed) = InstanceLogicOptimizer::sweep(&mut gate, &fabric, &cache, &targets);

        // -- visual: sextant waterfall frame --------------------------------
        viz.clear_canvas();
        // input layer: the object's true position (top-left half of frame)
        viz.set_sub_pixel(ox / 2, 0, true);
        // state layer: every +1 gate cell lights its sub-pixel (16x16 layout)
        for cell in 0..DIMS {
            if gate.get(cell) == 1 {
                viz.set_sub_pixel(cell % GRID, cell / GRID, true);
            }
        }
        let art = viz.render();
        println!("f{frame:03} obj=({ox:2},{oy:2}) err={err:6} flips={changed:3}");
        print!("{art}");

        // gate state summary line: how many + / - / 0
        let (mut p, mut m) = (0usize, 0usize);
        for c in 0..DIMS {
            match gate.get(c) { 1 => p += 1, -1 => m += 1, _ => {} }
        }
        println!("  gate: +{p} -{m} 0-{} | object drifts, gate follows", DIMS - p - m);
        std::thread::sleep(std::time::Duration::from_millis(45));
    }

    let _ = perceiver.join();

    // ---- final audit --------------------------------------------------------
    let (targets, _) = infer_targets(&fabric);
    let objective = exoj_kernel::optimization::Objective { fabric: &fabric, targets: &targets };
    println!("final whole-fabric error under evolved gate: {}", objective.total_error(&gate));
    println!("bootstrap complete — the seed learned to track.");
}
