# eos-seed — bootstrap seed of the Epigenetic Operating System

A minimal, deterministic PoC of the eOS execution loop in a clean Rust
workspace: a low-power, zero-VRAM loop that processes continuous embeddings
through a thin 2-bit packed ternary matrix, built on the `quilt-dba` and
`exoj` architectural paradigms. Runs anywhere — including GitHub Codespaces
— with `cargo run --release`.

## The tree

```
eos-seed/
├── .cargo/config.toml      # Zen 5 / x86-64 native acceleration profiles
├── Cargo.toml              # master workspace manifest (LTO, 1 codegen unit)
├── quilt_storage/          # Part 1: zero-copy binary file storage
│   └── src/
│       ├── fabric.rs       # memory-mapped continuous row-vector matrix
│       └── telemetry.rs    # non-blocking NMEA & sensor ingest layouts
├── exoj_kernel/            # Parts 2 & 3: discrete integer math & logic
│   └── src/
│       ├── ternary.rs      # 2-bit packed matrix execution engines
│       ├── optimization.rs # spreadsheet instance-logic coordinate steppers
│       └── inverse_physics.rs # reverse-engineering tracking compilers
├── eos_runtime/            # presentation & orchestration harness
│   └── src/
│       ├── main.rs         # E2E runtime broker, worker ring, loop
│       ├── broker.rs       # system-agnostic hardware capability detector
│       └── terminal.rs     # sub-character density visualizer (quadrants)
└── docs/LEDGER.md          # honest ledger: calibrations and dead ends
```

## The three organs

- **`quilt_storage::fabric`** — the shared matrix fabric. File-backed,
  memory-mapped (`memmap2`), append-only f32 rows. 64-byte aligned header
  tracks dims + row count on disk (bytes 24..64 reserved for tissue
  metadata). Appends copy straight into the map — zero intermediate buffers.
- **`exoj_kernel::ternary`** — every byte holds four base-3 states
  (`00` Muted, `01` Positive, `10` Blocked; `11` reserved). Evaluation reads
  raw bytes off the memory map and scores with integer add/sub only: the f32
  is quantized by bit-shift (`bits >> 24`, sign-preserved) — no FP
  multiplies on the hot path.
- **`exoj_kernel::optimization`** — the coordinate stepper. No backprop, no
  FP gradients: each pass walks cells in fixed order, flashes `{-1, 0, +1}`,
  and permanently commits whichever state minimizes total tracking error.
  Ties keep the current state (anti-oscillation hysteresis).

## Seams already cut (organic expansion)

- `telemetry.rs` — `$--DBT` depth ingest (NMEA → meters) and a fixed-width
  CSV sensor layout (width is the contract; the sensor adapts).
- `inverse_physics.rs` — the identity compiler: a fresh fabric derives its
  own targets from the log's structure (per-sign median identity score,
  snapped to the quantizer's reachable set). No human labeling.
- `broker.rs` — reads real host capabilities (cores, memory, arch), sizes
  the worker ring, best-effort core pinning.
- `terminal.rs` — 2×2 quadrant sub-character density rendering; a 2×3
  sextant (U+1FB00) variant is planned when terminal fonts catch up.

## The bootstrap loop (live)

`cargo run --release` now runs the full closed cycle: an async perception
worker streams a moving object across a 16x16 coordinate space over a
bounded crossbeam channel; every frame is appended zero-copy into the
256-dim memory-mapped fabric; the InstanceLogicOptimizer sweeps all 256
ternary switches against the last 48 historical vectors (novel rows =
signal, stale rows = background); and the HighDensitySubGridVisualizer
renders the gate's evolving state as a 6x packed sextant waterfall
(U+1FB00 legacy-computing block). The gate visibly learns to track the
drifting object — no gradients anywhere.

## Try it

```bash
cargo test --workspace --release   # 12 tests: fabric, packing, telemetry, stepper, broker
cargo run --release                # the click-and-play demo
```

Deterministic: fixed seed 2718, fixed walk order, tie-keeps-current
hysteresis. Identical output every run.

## The demo loop (what "epigenetic" means here)

Simulated sonar log: 96 rows × 32 dims, three classes whose signatures are
4 hot dims each (two positive, one negative). The gate starts from a seeded
random ternary state and evolves — no gradients — until its 32 switches
track targets the log inferred about itself. Latest seed run:

```
broker: 24 cores (8 in ring)
inverse physics: targets inferred ±256
initial gate error: 18219 -> converged: 15810
class 0 mean score: +110   class 1: +163   class 2: -163
```

The fabric's signature dims glow in the luminance view; the gate art shows
the switches the stepper committed. Sonar parsers and richer visualizers
plug into the reserved seams without touching the kernel.

Failures and calibrations are booked in `docs/LEDGER.md` — the first demo
run used targets unreachable under the quantizer's scale and sat at a bad
local minimum; the ledger keeps the lesson.
