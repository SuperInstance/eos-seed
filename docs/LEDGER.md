# eos-seed ledger — honest record of calibrations and dead ends

## 2026-09-27 — first calibration: target scale vs quantizer scale (booked)
- Symptom: demo converged instantly to a poor local minimum (error 193890
  → class-2 mean score +186 against a −64 target). The stepper was correct;
  the OBJECTIVE was unreachable.
- Cause: quantize() anchors to the float's exponent band, so every normal
  float maps to ~1000s at >>20. Targets of ±64 were below one step of any
  active cell — all-muted was better than anything the gate could express,
  and ties kept the gate from walking there.
- Fix: shift to >>24 (7-bit fixed point, unit dim ≈ 65); targets recalibrated
  to ±256. Error 285882 → 13110; class means +252 / +205 / −274.
- Lesson: in an integer-only loop, the quantizer IS the loss landscape.
  Choose targets inside its reachable set.
