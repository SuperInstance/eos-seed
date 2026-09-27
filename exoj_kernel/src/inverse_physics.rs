//! inverse_physics.rs — reverse-engineering tracking compilers.
//!
//! Where optimization asks "which switches track given targets?", inverse
//! physics asks the opposite: "given the log, what SHOULD the targets be?"
//! The seed ships one compiler: [`infer_targets`] — derive per-row target
//! scores from the fabric itself, so a fresh tissue bootstraps without a
//! human labeling anything.
//!
//! Deterministic, integer-only after quantization.

use quilt_storage::fabric::Fabric;

use crate::ternary::PackedTernary;

/// Infer per-row targets from the fabric's own structure.
///
/// Method (the "identity compiler"): score every row against the identity
/// gate (every cell Positive). Rows dominated by positive-valued dims get
/// large positive identity scores; negative-dominated rows go negative.
/// The class split is the SIGN of the identity score; the target magnitude
/// is the per-sign MEDIAN identity score, rounded to the nearest power of
/// two step so targets sit inside the quantizer's reachable set.
///
/// Returns (targets, sign_of_row) — targets[i] is what the stepper should
/// chase for row i.
pub fn infer_targets(fabric: &Fabric) -> (Vec<i64>, Vec<i8>) {
    let mut gate = PackedTernary::new(fabric.cols() as usize);
    for c in 0..gate.len() {
        gate.set(c, 1);
    }
    let mut identity: Vec<i64> = (0..fabric.rows())
        .map(|r| gate.score_row(fabric.row(r)))
        .collect();

    let mut pos: Vec<i64> = identity.iter().copied().filter(|s| *s >= 0).collect();
    let mut neg: Vec<i64> = identity.iter().copied().filter(|s| *s < 0).collect();
    pos.sort_unstable();
    neg.sort_unstable();
    let pos_t = pos.get(pos.len() / 2).copied().unwrap_or(0);
    let neg_t = neg.get(neg.len() / 2).copied().unwrap_or(0);

    // round to power-of-two-ish step (keeps targets reachable, honors scale)
    let step = |v: i64| {
        if v == 0 { return 0; }
        let m = v.abs();
        let p = 1i64 << (63 - (m as u64).leading_zeros()).min(62);
        (if m - p / 2 > p { p * 2 } else { p }) * v.signum()
    };
    let (pt, nt) = (step(pos_t), step(neg_t));

    let signs: Vec<i8> = identity.iter().map(|s| if *s >= 0 { 1 } else { -1 }).collect();
    let targets = identity
        .iter()
        .map(|s| if *s >= 0 { pt } else { nt })
        .collect();
    let _ = &mut identity;
    (targets, signs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_compiler_separates_sign_clusters() {
        let dir = std::env::temp_dir().join(format!(
            "eos-inv-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = Fabric::create(dir.join("inv.fab"), 8).unwrap();
        for i in 0..4 {
            let mut v = vec![0.0f32; 8];
            v[i] = 4.0; // positive-dominant rows
            f.append(&v).unwrap();
        }
        for i in 0..4 {
            let mut v = vec![0.0f32; 8];
            v[i] = -4.0; // negative-dominant rows
            f.append(&v).unwrap();
        }
        let (targets, signs) = infer_targets(&f);
        assert_eq!(targets.len(), 8);
        assert!(targets[..4].iter().all(|t| *t > 0));
        assert!(targets[4..].iter().all(|t| *t < 0));
        assert_eq!(signs[0], 1);
        assert_eq!(signs[7], -1);
    }
}
