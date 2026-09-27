# eos-seed — bootstrap seed of the Epigenetic Operating System

A minimal, deterministic PoC of the eOS execution loop in a clean Rust
workspace: a low-power, zero-VRAM loop that processes continuous embeddings
through a thin 2-bit packed ternary matrix, built on the `quilt-dba` and
`exoj` architectural paradigms. Runs anywhere — including GitHub Codespaces
— with `cargo run --release`.

## The mental model

Picture a spreadsheet that is only allowed three ink colors: black
(`Blocked`, opposes), gray (`Muted`, ignores), and white (`Positive`,
agrees). Every cell in that spreadsheet is a switch with exactly those three
states — no smooth gradient between them, ever. Learning is **not** "nudge
every weight a little in the gradient's direction" (backprop); it is "flip
one stuck switch to whichever of the three colors currently makes today's
row of numbers add up better, keep the switch if a flip doesn't help, and
never look back." That is the entire optimizer
(`exoj_kernel::optimization`) — a light-switch panel that anneals itself
into agreement with what its own memory-mapped rows say, using only integer
add/sub, never a floating-point multiply on the hot path.

```
   continuous embedding row (f32)
            │
            ▼  bit-shift quantize (bits >> 24, sign-preserved) — no FP multiply
   ┌────────────────────────────────┐
   │  N ternary switches             │  each byte packs 4 switches (2 bits each)
   │  { -1 Blocked, 0 Muted, +1 Pos } │
   └────────────────────────────────┘
            │  score = integer add/sub against the last 48 historical rows
            ▼
   tracking error (lower is better) ──▶ flip one switch, keep the flip only
            │                            if the error drops (else revert)
            ▼
   HighDensitySubGridVisualizer (sextant waterfall) — the gate visibly learns
```

Read the loop as one sentence: **a byte-packed panel of three-state
switches anneals itself against its own memory-mapped history, using only
integer arithmetic — no gradients, no floats on the hot path, no GPU.**

## A worked example

Take the shipped demo end to end. `cargo run --release` streams a 96-row ×
32-dim simulated sonar log through the loop above. Three classes hide in
that log, each with 4 "hot" dimensions (two positive, one negative) — the
log never tells the gate which dimensions matter; `inverse_physics.rs`
infers the targets from the log's own structure (per-sign median, snapped to
what the quantizer can actually represent).

```
$ cargo run --release
broker: 24 cores (8 in ring)
inverse physics: targets inferred ±256
initial gate error: 18219 -> converged: 15810
class 0 mean score: +110   class 1: +163   class 2: -163
```

Read the trace as one sentence: the gate started 18,219 units wrong, the
32-switch coordinate stepper walked it down to 15,810 with no gradient
anywhere, and by the end the three classes separate cleanly in the score
column (+110 / +163 / -163) — a real number a reader can reproduce with
`cargo run --release`, not a claim. (The full 96×32 run behind "The
bootstrap loop (live)" below is the same mechanism at the repo's actual
scale, not a toy.)

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

## What a reader learns

- **Learning does not require a gradient.** A fixed-order coordinate stepper
  over a tiny discrete state space (`{-1, 0, +1}`) can track a moving target
  using only integer add/sub and a keep-if-better rule.
- **Quantization can replace multiplication.** `bits >> 24` (sign-preserved)
  turns an f32 into a ternary state cheaply enough to run the whole loop
  without a single FP multiply on the hot path.
- **Targets can come from the data's own structure.** `inverse_physics.rs`
  derives what "correct" means from the log's per-sign medians instead of a
  human label — the fabric grades itself.
- **Zero-copy is a design constraint, not an optimization.** The
  memory-mapped fabric appends straight into the map; there is no
  intermediate buffer to get out of sync with disk.
- **Determinism is a feature you can point at.** Fixed seed, fixed walk
  order, tie-keeps-current hysteresis — the same input always produces the
  same converged state, and `cargo test --workspace --release` checks it.

<!-- QUILT:LINKS:START — generated from .quilt/links.yml by quilt-links.mjs. Do not edit by hand. -->
## Cross-pollination — the Reader's Fold

*Part of the **quilt** family. Under [Law 6](https://github.com/SuperInstance/jev-quilt), this repo carries no verdicts about its neighbors — only content-addressed pointers you fold under your own weights.*

**Grown on** — [quilt-dba](https://github.com/SuperInstance/quilt-dba), [exoj](https://github.com/SuperInstance/exoj)

**Provides** (fold these from here)
- `ternary-matmul-loop` — a zero-VRAM 2-bit packed ternary matrix execution engine + no-backprop coordinate-stepper optimizer, closing the loop end-to-end on continuous embeddings

**Related** (1-hop siblings — Law 7)
- [pong-quilt](https://github.com/SuperInstance/pong-quilt) — sibling "learning without backprop" teaching artifact — genetic algorithm there, ternary coordinate-stepper here

<sub>Regenerate: `node quilt-links.mjs` · Fleet map: [FLEET.md](https://github.com/SuperInstance/fleet-seeds/blob/main/FLEET.md)</sub>
<!-- QUILT:LINKS:END -->
