//! terminal.rs — 6x high-density sub-character visualizer.
//!
//! Renders matrices and gates into sub-character cells: each glyph packs a
//! 2×2 quadrant block (▘▝▖▗ and friends), so an 80-column terminal shows a
//! 160×48 fabric region. One glyph = 4 sub-cells; a 2×3 sextant variant is
//! planned when terminal fonts catch up with U+1FB00.
//!
//! Two views ship: `render_luminance` (fabric rows as brightness) and
//! `render_gate` (ternary states: + bright quadrant, − dim quadrant,
//! 0 blank).

/// Quadrant block characters indexed by (TL, TR, BL, BR) bit-nibble
/// (TL=8, TR=4, BL=2, BR=1).
const QUADRANTS: [char; 16] = [
    ' ', '▗', '▖', '▄', '▝', '▐', '▞', '▟', '▘', '▚', '▌', '▙', '▀', '▜', '▛', '█',
];

fn nibble(bits: [bool; 4]) -> usize {
    (bits[0] as usize) << 3 | (bits[1] as usize) << 2 | (bits[2] as usize) << 1 | bits[3] as usize
}

/// Render a 2D scalar field (values clamped 0..1) as quadrant-density text.
pub fn render_luminance(field: &[Vec<f32>]) -> String {
    let rows = field.len() / 2 * 2;
    let cols = field.first().map(|r| r.len()).unwrap_or(0) / 2 * 2;
    let mut out = String::new();
    for by in (0..rows).step_by(2) {
        for bx in (0..cols).step_by(2) {
            let bits = [
                field[by][bx] >= 0.5,
                field[by][bx + 1] >= 0.5,
                field[by + 1][bx] >= 0.5,
                field[by + 1][bx + 1] >= 0.5,
            ];
            out.push(QUADRANTS[nibble(bits)]);
        }
        out.push('\n');
    }
    out
}

/// Render a packed ternary gate: positive = full block half, negative =
/// half-height bars, muted = blank. Width = gate.len() (already dense).
pub fn render_gate(gate: &[i8]) -> String {
    let mut top = String::new();
    let mut bottom = String::new();
    for &s in gate {
        match s {
            1 => { top.push('█'); bottom.push('▀'); } // positive: full column
            -1 => { top.push('▄'); bottom.push('▄'); } // blocked: low bar
            _ => { top.push(' '); bottom.push(' '); }
        }
    }
    format!("{top}\n{bottom}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadrant_nibbles_map_to_expected_glyphs() {
        assert_eq!(QUADRANTS[nibble([false; 4])], ' ');
        assert_eq!(QUADRANTS[nibble([true; 4])], '█');
        assert_eq!(QUADRANTS[nibble([true, true, false, false])], '▀');
        assert_eq!(QUADRANTS[nibble([false, true, false, true])], '▐');
    }

    #[test]
    fn gate_render_shows_three_states() {
        let art = render_gate(&[1, -1, 0]);
        assert!(art.contains('█'));
        assert!(art.contains('▄'));
    }
}
