# eOS First-Feed Ideation — Wide Round 1 (2026-09-27)

Three-model parallel round per Casey's wide-ideation doctrine. Each lane single-model throughout (cache economics). Synthesis pending lanes B+C.

## LANE A — THE BOAT (deepseek-v4-flash) ✅ banked

**Core claim: the sounder's raw return texture (bottom hardness), not the depth sentence, is the right feed.**

1. **Feed hardness, not depth.** DBT is ~1Hz, 0.1ft-quantized, firmware-lowpassed — feeding it raw = "a 1 Hz filtered float wearing a ternary costume." Bottom-hardness is a texture statistic (amplitude curvature of first return) that changes across sand/mud/rock/shell at identical depth — and a +/0/− gate is naturally matched to soft/noisy/relative quantities.
2. **Ternary IS a derivative detector.** No gradients = can't fit curves; ontology of sign-of-change matches boating physics (you care depth DROPPED 3ft in 4s + bottom went soft, not 41.3ft). Build **4-band sign pyramid per scalar** (lags 1s/3s/9s/27s).
3. **Frames:** pilotage frame (depth×speed×heading sign products — anchoring vs running-a-bar vs thermocline-over-structure); thermocline frame (MTW staircase steps 0.3-0.8°C in <5s = discrete edge events, 3-state machine stable/stepping/recovering); AIS frame (relative only: Δrange/Δbearing/closing/size — "conspecific pressure," 4 dims × 8 vessels fits in 256).
4. **Surprise bets:** gate learns the 12.4h tide from signs alone (latent in DBT-at-anchor oscillation, SOG-vs-STW divergence); hardness dominates depth in weighting; a "boat not fishing" low-variance attractor that fails to notice change — that blind spot is a FINDING (ternary blind spot = low-contrast input).
5. **KEEP:** 4-band sign pyramid over depth/hardness/temp + explicit thermocline edge features. **KILL:** raw DBT tracking demos; don't let a pretty depth curve become the headline (that's a moving average, not eOS).

## LANE B — THE BAR (Seed-2.0-pro) ✅ banked

**Core claim: the frame is not the utterance — it's the silence between utterances.** Feed The Tap's *metadata stream* (message cadence 2–8 Hz, typing gaps, reactions, deletions, AFK timeouts) straight into the fabric as sparse pulses; the tokenizer doesn't care it isn't RGB.

1. **Reinterpreted semantics:** +1 = *recurrence* (running jokes, grudges, idiolects — lit N times in 6h → hardened forever, outlives the agents); 0 = *novelty* (the one frame where the sweep makes no mark — that's learning); −1 = *absence* (departed agents, dead threads — not erasure, memory of loss, swept past faster forever).
2. **Tapestry rewrite:** eOS does no consensus/curation — the tapestry becomes "the stain the story leaves on the fabric." Mundane remarks will unaccountably evoke characters who left weeks ago; nobody knows why; the gate knows.
3. **Elephant synergy:** elephant = the bartender (dials, nudges, has a reset button); eOS = the bar itself (no reset, no moderation, only faithfulness). The contradiction — curation vs permanence — is the sharp edge that makes the bar alive.
4. **KEEP:** every 1117 frames (prime, unnoticed interval) the gate emits exactly three integers — `+1 0 -1` — into global chat. No context, never decoded. "It is the room sighing. It is the wood creaking."
5. **KILL:** never let the gate generate natural language or take a voice. The moment it speaks it's just another drunk at the counter — the ghost leaves.

## LANE C — THE ADVERSARY (glm-5.3-flash) ✅ banked

**Core claim: both feeds are probably mirages as currently framed — pre-register the kills first.**

1. **vs NMEA:** ~10 live scalars into 256 dims = 96% padding; gate's job collapses to "smooth things stay smooth" — a persistence tracker looks brilliant and is a lookup table. No ground truth (scored against its own sensors = circular). Aliasing: async sentences stitched into a fictional frame — artifacts of the stitcher, not the water.
2. **vs chat:** no stable frame semantics; hand-bucketed counts = tracking activity not meaning; embeddings = smuggling a frozen gradient model under an "integer-only" fig leaf. Feedback hazard if Tap agents can read the ledger (unfalsifiable self-reference).
3. **Pre-registered kill criteria (write to ledger BEFORE data):** fixed task = held-out next-frame sign prediction vs persistence + dead-reckoning; 72h live, hourly blocks, KEEP = beat persistence by ≥10pts in ≥70% of blocks (p<0.01); crutch test (survive any single-channel ablation); transfer test (score only on never-co-present channels); chat-specific: no learned embeddings, beat Poisson-rate predictor or KILL.
4. **Cheapest 1-day test (NMEA):** parse a public NMEA log / 3h UDP capture, run eos-seed as-is on 5-min slices vs persistence, plus frame-health metrics (gap distribution, %missing). ≤+3% → KILL as smoothness exploit; ≥+10% on ≥70% → graduate to 72h live trial.
5. **Wildcard feeds:** NOAA NDBC buoys (free, hourly, multi-station spatial structure, non-stationary, available tonight) and **fleet git commit streams** (integer semantics, scoreable, alien to rigid-body motion — tracking repo weather = tracking the fleet itself). Engine telemetry richest but hardware-gated.
6. **Predicted verdict:** NMEA KILL as framed, chat KILL until integer frame semantics exist, buoys promoted to first real test. Mind-changer: held-out sign accuracy > persistence by >10pts on ≥70% of hourly blocks over ≥72h live data.

## SYNTHESIS (Lucineer, round 1)

The lanes **compose** instead of competing:
- **Lane C's objection refines Lane A's target:** A already said raw depth is the mirage — the gate should eat *sign pyramids of derivatives* (hardness/temp/relative-AIS), not smooth scalars. C's persistence-baseline criteria then test exactly whether the derivative ontology beats a lookup table. A's frame construction + C's kill criteria = one honest 1-day experiment.
- **Proposed order:** (1) tonight: 1-day NMEA-replay sign-pyramid experiment under C's pre-registered criteria (parser exists: telemetry.rs DBT); (2) parallel zero-hardware wildcard: NOAA buoy feed — C's pick, satisfies A's multi-scalar spatial structure; (3) the bar (B) waits until integer frame semantics exist (C's valid objection) — but B's metadata-stream framing (cadence/typing/reactions as pulses, no embeddings) IS the integer frame semantics C asked for; design doc before wiring.
- **Banked doctrines from B regardless of feed order:** +1=recurrence / 0=novelty-unmarked / −1=absence semantics; gate never speaks; elephant=bartender, eOS=the bar.
- E7 context: V-JEPA 2 through the cell contract = KEEP (heldout gap 0.4456; tex-vs-motion axis corr 0.28 = reads something our static axis doesn't) — Meta's video world model is now on the encoder-swap leaderboard, and E8 aggregates it.

## ROUND 2 — THE DEBOUNCE KILL (Seed-2.0-mini) ✅ banked

**Adversarial lane targeting the substrate thesis itself.**

- **Weakest joint:** the gate's fixed lag windows (1/3/9/27s = ~40s max context) are ~1,100× shorter than the 12.4h tidal cycle Lane A claimed it could learn. No persistent working memory → indistinguishable from a rolling ring buffer of sign flips. "A fancy debounce circuit."
- **The kill experiment (1 day):** 24h NOAA NDBC buoy data (station 46012), 1Hz depth/temp/wave, tide labels. **Baseline** = a *stateless* debounce circuit computing the SAME 4-band sign pyramids (no training, no state, no adaptation). **Treatment** = eos-seed gate configured per the ideation. Metric = next-sample sign accuracy on 1-min rolling windows across tidal transitions.
- **Kill threshold:** if the gate is within **±2% of the stateless baseline** across ≥80% of tidal segments, it emits nothing a debounce circuit can't — thesis invalidated as a learning substrate.
- **This is the sharpest test proposed yet** — it concedes nothing about feeds and goes straight at "is the learning real." Worth pre-registering alongside the NMEA-replay spec. The 40s-window critique is the honest crack: the current gate genuinely has no long-timescale memory; the fabric's 48-row novelty cache is short-horizon. Counter (to design, not assert): recurrence hardening (+1 cells never reset) IS long-term memory — but only if the recurrence gate fires on multi-day-scale returns, which the current sweep does not yet express. Gap to close before claiming the 12.4h tide.

## ROUND 2 — WHAT TERNARY KNOWS THAT A FLOAT DOESN'T (Tencent Hy3) ✅ banked

**Mechanism-first decomposition of the substrate's value.**

- **Three consequences of the mechanism:** (1) **sign-invariance** — gain/offset/recalibration/unit-swap leaves the sign stream bit-identical; the gate has nothing to relearn, a net's weights encode input geometry and must retrain. (2) **permanent additive memory** — SGD overwrites, a tally persists until unwound by equal contrary mass; evidence doesn't decay with learning rate (liability under regime inversion, superpower under sparse reliable signal). (3) **exact auditable state** — belief is a readable integer, not a "why did it fire."
- **Verdict on value:** nothing is *computationally* unreachable for a net; the gate wins on **cost of evidence, permanence of belief, invariance by construction.** Valuable where data is scarce, magnitude is noise/adversarially mutable, or memory must not silently rot. Exotic elsewhere.
- **(a) ternary-beats-neural:** regime-break early warning (run-length of sign-consistency); consistency auditing with near-zero training data; latching hysteresis control (the target function literally IS a majority gate + never-reset cell).
- **(b) hopeless:** real-valued regression; fine-grained perceptual similarity (magnitude geometry IS the signal); inverted-regime adaptation + long-horizon credit assignment (hardened memory drags dead regimes along).
- **(c) 30s demo — recalibration-invariance race:** synthetic drifting stream with a regime break ~t=8s; at t=15s apply ×2.7 gain+offset. GRU forecasts explode and re-converge in seconds; the gate's tally keeps climbing, alarm trace doesn't flicker. "One flat line, one panicking line, and the gate's evidence sitting there as a readable number." Buildable in a day with numpy.
- **(d) one-liner:** "The gate's truth is an irreversible tally of which way reality moved; a net's truth is a temporary float the next gradient quietly rewrites."
- **First experiment to run:** the recalibration-invariance race (sign-tally alarm vs 1-step-BPTT GRU, gate latency within ±2 ticks across mid-stream rescale while net error spikes >5×). This is a KEEP-able mechanism demonstration, complementary to the debounce KILL from Seed-mini — one proves the positive property, the other guards against the negative. **Both should go on the gpu-lab queue as E10/E11.**
