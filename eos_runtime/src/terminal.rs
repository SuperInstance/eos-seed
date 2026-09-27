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

#[allow(dead_code)]
/// Quadrant block characters indexed by (TL, TR, BL, BR) bit-nibble
/// (TL=8, TR=4, BL=2, BR=1).
const QUADRANTS: [char; 16] = [
    ' ', '▗', '▖', '▄', '▝', '▐', '▞', '▟', '▘', '▚', '▌', '▙', '▀', '▜', '▛', '█',
];

#[allow(dead_code)]
fn nibble(bits: [bool; 4]) -> usize {
    (bits[0] as usize) << 3 | (bits[1] as usize) << 2 | (bits[2] as usize) << 1 | bits[3] as usize
}

/// Render a 2D scalar field (values clamped 0..1) as quadrant-density text.
#[allow(dead_code)]
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
#[allow(dead_code)]
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

/// The 6x packed sub-character raster engine (eOS spec): every text cell is
/// a 2x3 sub-pixel sextant. Glyph mapping per the eOS design doc: bitmask 0
/// = blank, 0b111111 = full block, else U+1FB00 + (bitmask - 1). Terminal
/// fonts without the Symbols-for-Legacy-Computing block will show fallback
/// glyphs — the raster itself stays correct and portable.
pub struct HighDensitySubGridVisualizer {
    text_cols: usize,
    text_rows: usize,
    sub_cols: usize,
    sub_rows: usize,
    sub_pixel_buffer: Vec<bool>,
}

impl HighDensitySubGridVisualizer {
    pub fn new(text_cols: usize, text_rows: usize) -> Self {
        let (sub_cols, sub_rows) = (text_cols * 2, text_rows * 3);
        HighDensitySubGridVisualizer {
            text_cols,
            text_rows,
            sub_cols,
            sub_rows,
            sub_pixel_buffer: vec![false; sub_cols * sub_rows],
        }
    }

    pub fn clear_canvas(&mut self) {
        self.sub_pixel_buffer.fill(false);
    }

    #[inline]
    pub fn set_sub_pixel(&mut self, sub_x: usize, sub_y: usize, active: bool) {
        if sub_x < self.sub_cols && sub_y < self.sub_rows {
            self.sub_pixel_buffer[sub_y * self.sub_cols + sub_x] = active;
        }
    }

    #[inline]
    fn lookup_sextant_char(&self, bitmask: u8) -> char {
        if bitmask == 0 { return ' '; }
        if bitmask == 0b111111 { return '█'; }
        std::char::from_u32(0x1FB00 + (bitmask as u32 - 1)).unwrap_or('.')
    }

    /// Render the packed sub-grid into a string (idiomatic + testable;
    /// the caller owns printing).
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(self.text_rows * (self.text_cols + 1));
        for ty in 0..self.text_rows {
            for tx in 0..self.text_cols {
                let bx = tx * 2;
                let by = ty * 3;
                let bit = |dx: usize, dy: usize| -> u8 {
                    if self.sub_pixel_buffer[(by + dy) * self.sub_cols + bx + dx] {
                        1
                    } else {
                        0
                    }
                };
                let bitmask = bit(0, 0) | bit(1, 0) << 1 | bit(0, 1) << 2
                    | bit(1, 1) << 3 | bit(0, 2) << 4 | bit(1, 2) << 5;
                out.push(self.lookup_sextant_char(bitmask));
            }
            out.push('\n');
        }
        out
    }

    #[allow(dead_code)]
pub fn dims(&self) -> (usize, usize, usize, usize) {
        (self.text_cols, self.text_rows, self.sub_cols, self.sub_rows)
    }
}

#[cfg(test)]
mod high_density_tests {
    use super::*;

    #[test]
    fn sextant_raster_roundtrip() {
        let mut v = HighDensitySubGridVisualizer::new(2, 1); // 4x3 sub-pixels
        v.set_sub_pixel(0, 0, true);
        v.set_sub_pixel(3, 2, true);
        let art = v.render();
        let lines: Vec<&str> = art.lines().collect();
        assert_eq!(lines.len(), 1);
        // cell (0,0): TL sub-pixel set -> bitmask 0b000001 -> U+1FB00
        // cell (1,0): BR sub-pixel set -> bitmask 0b100000 -> U+1FB00+30
        let chars: Vec<char> = lines[0].chars().collect();
        assert_eq!(chars[0], std::char::from_u32(0x1FB00).unwrap());
        assert_eq!(chars[1], std::char::from_u32(0x1FB00 + 31).unwrap());
    }

    #[test]
    fn full_block_and_bounds() {
        let mut v = HighDensitySubGridVisualizer::new(1, 1);
        for y in 0..3 { for x in 0..2 { v.set_sub_pixel(x, y, true); } }
        v.set_sub_pixel(99, 99, true); // out of bounds: ignored
        assert_eq!(v.render().trim(), "█");
    }
}
