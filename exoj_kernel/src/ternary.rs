//! ternary.rs — the 2-bit packed gating array.
//!
//! Every byte holds exactly four base-3 states, 2 bits each:
//!   `00` = 0  Muted
//!   `01` = +1 Positive Alignment
//!   `10` = -1 Blocked / Negative
//!   `11` = reserved (never written by this kernel)
//!
//! Bit layout within a byte: cell 0 = bits 1..0 (least significant), cell 1 =
//! bits 3..2, cell 2 = bits 5..4, cell 3 = bits 7..6.
//!
//! The evaluation loop reads vectors straight from a raw f32 byte slice (as
//! served by quilt_storage's memory map) and computes an alignment score with
//! integer-only addition and subtraction — no FP multiplies anywhere on the
//! hot path.

pub const STATE_MUTED: i8 = 0;
pub const STATE_POSITIVE: i8 = 1;
pub const STATE_BLOCKED: i8 = -1;

/// 2-bit encodings.
const BITS_MUTED: u8 = 0b00;
const BITS_POSITIVE: u8 = 0b01;
const BITS_BLOCKED: u8 = 0b10;

/// A packed ternary gate vector over `len` cells.
#[derive(Clone)]
pub struct PackedTernary {
    bytes: Vec<u8>,
    len: usize,
}

impl PackedTernary {
    /// A gate of `len` cells, every state Muted.
    pub fn new(len: usize) -> PackedTernary {
        PackedTernary { bytes: vec![0u8; len.div_ceil(4)], len }
    }

    /// Deterministic pseudo-random init (xorshift64*) — no external RNG.
    pub fn seeded(len: usize, seed: u64) -> PackedTernary {
        let mut g = PackedTernary::new(len);
        let mut s = seed | 1;
        for i in 0..len {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            let r = s.wrapping_mul(0x2545F4914F6CDD1D);
            let state = match r & 0b11 {
                0 | 3 => STATE_MUTED,
                1 => STATE_POSITIVE,
                _ => STATE_BLOCKED,
            };
            g.set(i, state);
        }
        g
    }

    #[inline]
    pub fn len(&self) -> usize { self.len }

    #[inline]
    pub fn is_empty(&self) -> bool { self.len == 0 }

    #[inline]
    pub fn get(&self, i: usize) -> i8 {
        assert!(i < self.len);
        let byte = self.bytes[i / 4];
        let bits = (byte >> ((i % 4) * 2)) & 0b11;
        match bits {
            BITS_POSITIVE => STATE_POSITIVE,
            BITS_BLOCKED => STATE_BLOCKED,
            _ => STATE_MUTED,
        }
    }

    #[inline]
    pub fn set(&mut self, i: usize, state: i8) {
        assert!(i < self.len);
        let bits = match state {
            STATE_POSITIVE => BITS_POSITIVE,
            STATE_BLOCKED => BITS_BLOCKED,
            _ => BITS_MUTED,
        };
        let shift = (i % 4) * 2;
        self.bytes[i / 4] = (self.bytes[i / 4] & !(0b11 << shift)) | (bits << shift);
    }

    pub fn as_bytes(&self) -> &[u8] { &self.bytes }

    /// Quantize one f32 (given as raw bits) to a small integer, sign- and
    /// order-preserving, using shifts only. The IEEE-754 bit pattern of a
    /// non-negative float is monotonically ordered, so shifting the low
    /// sign bit off and taking the top exponent-plus-mantissa bits gives a
    /// stable integer proxy: q in 0..=2047 for positives, negated for
    /// negatives. Zero maps to zero.
    #[inline]
    pub fn quantize_bits(bits: u32) -> i32 {
        let sign = bits >> 31;
        let mag = (bits & 0x7FFF_FFFF) >> 24; // exponent + top 4 mantissa bits, 7-bit scale
        let q = mag as i32;
        if sign == 1 { -q } else { q }
    }

    #[inline]
    fn quantize(v: f32) -> i32 { Self::quantize_bits(v.to_bits()) }

    /// Alignment score of one row against this gate: sum over columns of
    /// gate_state × quantized(value), computed with integer add/sub only.
    ///
    /// `row_bytes` is the raw little-endian f32 bytes of the row — exactly
    /// what the fabric's memory map serves. No FP arithmetic occurs.
    pub fn score_row_bytes(&self, row_bytes: &[u8]) -> i64 {
        debug_assert_eq!(row_bytes.len(), self.len * 4);
        let mut acc: i64 = 0;
        for cell in 0..self.len {
            let state = self.get(cell);
            if state == STATE_MUTED {
                continue; // muted cells never touch the data
            }
            let b = &row_bytes[cell * 4..cell * 4 + 4];
            let bits = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            let q = Self::quantize_bits(bits);
            if state == STATE_POSITIVE {
                acc += q as i64;
            } else {
                acc -= q as i64;
            }
        }
        acc
    }

    /// Convenience overload: score an &[f32] row.
    pub fn score_row(&self, row: &[f32]) -> i64 {
        let mut acc: i64 = 0;
        for cell in 0..self.len {
            match self.get(cell) {
                STATE_MUTED => {}
                STATE_POSITIVE => acc += Self::quantize(row[cell]) as i64,
                STATE_BLOCKED => acc -= Self::quantize(row[cell]) as i64,
                _ => unreachable!(),
            }
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_states_per_byte_roundtrip() {
        let mut g = PackedTernary::new(8);
        let states = [STATE_POSITIVE, STATE_BLOCKED, STATE_MUTED, STATE_POSITIVE,
                      STATE_BLOCKED, STATE_MUTED, STATE_POSITIVE, STATE_BLOCKED];
        for (i, s) in states.iter().enumerate() {
            g.set(i, *s);
        }
        // 8 cells -> exactly 2 bytes
        assert_eq!(g.as_bytes().len(), 2);
        for (i, s) in states.iter().enumerate() {
            assert_eq!(g.get(i), *s);
        }
    }

    #[test]
    fn quantize_is_sign_and_order_preserving() {
        // 7-bit granularity: values in the same octave can collide (1.0 vs
        // 0.5 both -> 63); ordering is guaranteed only ACROSS the shift.
        assert_eq!(PackedTernary::quantize(0.0), 0);
        assert!(PackedTernary::quantize(2.0) > PackedTernary::quantize(1.0));
        assert!(PackedTernary::quantize(-2.0) < PackedTernary::quantize(-1.0));
        assert_eq!(PackedTernary::quantize(2.0), -PackedTernary::quantize(-2.0));
    }

    #[test]
    fn score_matches_reference() {
        let mut g = PackedTernary::new(4);
        g.set(0, STATE_POSITIVE);
        g.set(1, STATE_BLOCKED);
        g.set(2, STATE_MUTED);
        g.set(3, STATE_POSITIVE);
        let row = [4.0, 2.0, 999.0, -1.0];
        let expect = PackedTernary::quantize(4.0) as i64
            - PackedTernary::quantize(2.0) as i64
            + PackedTernary::quantize(-1.0) as i64;
        assert_eq!(g.score_row(&row), expect);
        // byte-slice path (as the fabric serves it) must agree exactly
        let bytes: Vec<u8> = row.iter()
            .flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(g.score_row_bytes(&bytes), expect);
    }
}
