# eos-seed — bootstrap seed of the Epigenetic Operating System

A minimal, deterministic PoC of the eOS execution loop in a clean Rust
workspace: a zero-VRAM loop that processes continuous embeddings through a
thin 2-bit packed ternary matrix, built on the `quilt-dba` and `exoj`
architectural paradigms. Runs anywhere — including GitHub Codespaces — with
`cargo run --release`.

## The three organs

| crate | organ | what it does |
|---|---|---|
| `quilt_storage` | Shared Matrix Fabric | file-backed, memory-mapped append-only f32 array (`memmap2`); 64-byte aligned header tracks dims + row count on disk; appends copy straight into the map, zero intermediate buffers |
| `exoj_kernel` | 2-Bit Packed Gating Array | every byte holds four base-3 states (`00` Muted, `01` Positive, `10` Blocked); evaluation reads raw bytes off the memory map and scores with integer add/sub only — no FP multiplies on the hot path |
| `instance_logic` | Coordinate Stepper | deterministic gradient-free optimization: walks cells cell-by-cell, flashes `{-1, 0, +1}`, permanently commits whichever state lowers total tracking error across the stored log |

## Try it

```bash
cargo test --workspace --release   # 6 tests: fabric roundtrip, packing, determinism
cargo run --release                # the click-and-play demo (simulated log vectors)
```

Deterministic: fixed seed 2718, fixed walk order, tie-keeps-current
hysteresis. Identical output every run.

## The demo loop (what "epigenetic" means here)

Simulated sonar log: 96 rows × 32 dims, three classes whose signatures are
4 hot dims each (two positive classes, one negative). The gate starts from
a seeded random ternary state and evolves — no gradients, no backprop —
until its 32 switches track the class targets. Measured on the seed run:

```
initial error: 17825  ->  converged: 13110
class 0 mean score: +252   (target +256)
class 1 mean score: +205   (target +256)
class 2 mean score: -274   (target -256)
```

The integer quantizer (`f32` bits >> 24, sign-preserved, octave
granularity documented in the kernel) is the contract between the
continuous log and the ternary fabric.

## Roadmap (plugin seams already reserved)

- sonar parsers → new rows via `Fabric::append`
- ASCII visualizer → reads the fabric + renders the gate as text
- multi-gate fabrics (one packed array per tissue)
- header growth: `bytes 24..64` reserved for future tissue metadata

Failures and calibrations are booked in-repo: the first demo run used
targets unreachable under the quantizer's scale and sat at a bad local
minimum — the ledger keeps the lesson (see `docs/LEDGER.md`).
