# NMEA-REPLAY-SPEC — eos-seed first-feed experiment (E8)

**Status:** pre-registration draft. Criteria in §5 are frozen **before** any data is scored. Commit hash of this file + timestamp go into `docs/LEDGER.md` before the first log is opened.

**One-line goal:** test whether the ternary gate, fed a 4-band sign pyramid built from a real NMEA replay, beats a persistence lookup table at next-frame sign prediction — under kill criteria written by the adversary lane.

**Honesty frame (from Lane C):** the expected default outcome is KILL. A KILL with clean numbers is a publishable ledger entry; a KEEP that survives pre-registered criteria is the surprise, not the goal.

---

## 1. Data

- **Source:** one or more public NMEA-0183 logs containing `$--DBT` (depth) and `$--MTW` (water temp), ideally ≥24 h total. Short logs (≥3 h each) are acceptable; then ≥20 distinct logs are required so hourly blocks can be pooled.
- **Third channel substitution:** hardness (first-return amplitude curvature) does **not exist** in standard NMEA sentences. Replay slot-3 = **SOG** (`$--VHW`/`$--VBW` kn field). Hardness replaces SOG in the same layout slot at the live trial (§5 Tier 1) — this doubles as the transfer-test channel.
- **Frame-health metrics are first-class output** (Lane C): per-channel gap distribution, % missing ticks, longest dropout. Report before any accuracy number. If a channel is >50% missing in a block, the block is marked DEGRADED and excluded from the KEEP/KILL arithmetic (count reported separately).

## 2. Frame construction — 4-band sign pyramid

**Tick grid.** Canonical 1 Hz grid over the log's wall-clock span. Per tick, per channel: last raw value received within that 1 s bucket (no interpolation — interpolation manufactures the smoothness we are testing for). Missing bucket → GAP flag, previous value carries forward **for differencing only**, never for scoring.

**Sign bands.** For channel c ∈ {depth-m (DBT), temp-°C (MTW), SOG-kn}, lag L ∈ {1, 3, 9, 27} s:

```
d_c,L(t) = s_c(t) − s_c(t−L)
σ_c,L(t) = +1 if d > +τ_c ;  −1 if d < −τ_c ;  0 otherwise
```

Dead-zone thresholds τ_c absorb sensor quantization: **τ_depth = 0.10 m, τ_temp = 0.05 °C, τ_SOG = 0.10 kn.** Fixed before data; changing them post-hoc voids the run.

**256-dim row layout** (f32 values in {−1, 0, +1} + status band; fabric width stays 256):

| Cols | Content |
|---|---|
| 0–11 | Instantaneous pyramid: σ_c,L(t), 3 channels × 4 lags (c-major, lags inner) |
| 12–251 | History tape: the same 12 signs for t−1 … t−20 (20 ticks × 12) — gives the sweep temporal context with no recurrence |
| 252–254 | Gap flags: depth/temp/SOG missing-in-last-10s indicators (0/1) |
| 255 | Bias cell, constant +1 |

Channels present in replay: 12 sign dims + 3 flags live; ~96% of dims are tape/flags, but **live-dim count and fill % are reported in every result table** so the Lane-C padding objection stays visible, not hidden.

## 3. Task and scoring

**Task:** held-out next-tick sign prediction — predict σ_c,L(t+1) for all 12 sign dims.

**Prediction = gate state.** No output head, no argmax layer: the ternary state of the cell at each (c,L) position of the gate **is** the prediction (+1/0/−1). This keeps the loop integer-only end-to-end and makes the task exactly the gate's native ontology.

**Score:** three-class exact-match accuracy per (c,L) dim, averaged over the 12 dims, over non-GAP ticks only. Reported per hourly block (logs ≥24 h) or per 5-min slice pooled across logs (short-log mode). Block-level gate accuracy minus block-level baseline accuracy = the margin, in points.

**Protocol:** chronological split — first 70% of ticks = train (gate sweeps here), last 30% = test (gate frozen, sweep off, states read out). No shuffling.

## 4. Baselines — what "persistence" means for sign prediction

1. **Persistence (primary):** at each tick t, predict σ_c,L(t+1) = σ_c,L(t) for every dim. Zero parameters, zero training. This is the "smooth things stay smooth" lookup table the adversary lane says will match the gate. The bar to beat.
2. **Majority class (sanity):** predict the modal sign of the training set for each (c,L) dim. If the gate fails to beat *this*, the sweep learned nothing at all — automatic KILL regardless of the persistence margin.

Both baselines score on identical ticks, identical GAP exclusions, identical blocks as the gate. One scorer, three columns.

## 5. Pre-registered kill criteria

### Tier 0 — replay (this experiment, 1 day)
- **KILL** if mean margin over blocks ≤ **+3 pts** → "persistence exploit / smoothness," close the lane.
- **AMBIGUOUS** if margin ∈ (+3, +10) pts or beats +10 pts in <70% of blocks → one rerun permitted with a *different* log only (never the same data twice); still ambiguous after rerun = KILL.
- **KEEP / graduate** if gate beats persistence by **≥10 pts in ≥70% of blocks**, with p < 0.01 (block-level sign test vs 50% win rate against persistence, two-sided).
- **Crutch test (Tier 0.5, same day):** zero out one channel's 4 dims + tape + flag at test time, per channel. Score must stay within 5 pts of the full-frame margin on ≥2 of 3 ablations. Failing one ablation channel = that channel was a crutch → note it; failing 2+ = KILL (the gate didn't learn the water, it learned one sensor's rhythm).

### Tier 1 — live 72 h trial (graduation gate, hardware days)
- ≥10 pts over persistence on ≥70% of **hourly** blocks across ≥72 h live capture, p < 0.01.
- **Transfer test:** hardness occupies the SOG slot, a channel never co-present with depth/temp in any training data. Score hardness dims alone: must beat persistence on them by ≥5 pts, else KEEP is downgraded to "depth/temp only" and hardness lane closes.

## 6. Cheapest implementation path

**Reused unchanged:**
- `quilt_storage::fabric` — create 256-wide fabric; file-is-database discipline, append-only rows. No edits.
- `quilt_storage::telemetry::DbtIngest::parse` — DBT→meters exactly as shipped.
- `exoj_kernel::ternary::PackedTernary` + `optimization::{run_pass, run_to_convergence}` — the sweep runs over the fabric as-is; train = sweep on ticks 0..70%, test = read gate states frozen.
- `shm.rs`, `video.rs`, runtime broker — untouched.

**New (all small, all in existing patterns):**
1. `telemetry.rs`: two parse functions cloned from `DbtIngest::parse` — MTW (field 1, °C) and SOG (VHW/VBW knots field). Same return-Option discipline.
2. `telemetry.rs`: `SignPyramidLayout` — owns the 1 Hz grid, τ dead-zones, gap flags, history tape; `ingest(tick_values) → row[256]` append. Sister of `CsvLayout`.
3. `eos_runtime` (or a `examples/nmea_replay.rs` bin): replay driver — read log file line-by-line → parsers → layout → fabric; at 70% boundary run sweep, then dump per-tick (gate prediction 12, ground truth 12, gap flags) to a sidecar CSV.
4. Scorer: standalone script (Python or awk) over the sidecar CSV — per-block accuracy for gate/persistence/majority, block table, sign test, ablation reruns. Analysis lives off the kernel; integer-only purity applies to the hot path, not the report generator.

**Effort estimate:** ~200 lines Rust + ~100 lines script. The only conceptual novelty is the layout; everything else is plumbing that already exists.

## 7. Go/no-go decision rule (KILL = publishable)

Before the first run: this spec's §5 goes verbatim into `docs/LEDGER.md` with commit hash and date. After scoring, exactly one entry is appended, chosen from:

- **KILL(replay-margin):** "gate − persistence ≤ +3 pts (measured: X). Sign pyramid on DBT/MTW/SOG is a smoothness exploit. Boat feed needs non-NMEA texture (hardness) or dies." → closes lane A's replay claim; buoy wildcard (Lane C) becomes first real test.
- **KILL(crutch):** margin passed but ≥2 ablations failed → gate leaned on one sensor; finding = which dim carries the gate.
- **KILL(majority):** worse than modal sign → sweep learned nothing from water; return to calibration ledger.
- **KEEP(replay):** margin + significance + crutch all passed → Tier 1 72 h live trial gets scheduled with the transfer test pre-registered above.
- **AMBIGUOUS→KILL:** rerun clause exhausted.

Every entry carries the full block table, fill %, gap stats, and the exact spec commit it was scored under. A KEEP without its ledger entry is not a result.

---

## Build order

1. Ledger-first: commit this spec, append §5 + hash to `docs/LEDGER.md` — no data opened before this lands.
2. Acquire candidate NMEA logs; run frame-health report (gap/fill table) and pick the ≥24 h or ≥20-log set.
3. Add MTW + SOG parsers and `SignPyramidLayout` to `telemetry.rs` (unit tests: τ dead-zone edges, tape shift, gap flags).
4. `nmea_replay` driver: log → fabric → sweep at 70% → sidecar CSV.
5. Scorer script: three-column block table + sign test + ablation matrix → write the single KEEP/KILL ledger entry.
