//! telemetry.rs — non-blocking NMEA & sensor ingest layouts.
//!
//! The first sonar-parser seam: turns raw instrument sentences into
//! fabric rows. Two layouts ship in the seed:
//!
//! - [`DbtIngest`]: NMEA `$--DBT` depth-below-transducer sentences
//!   (feet, fathoms, meters) → single-column depth fabric (meters)
//! - [`CsvLayout`]: generic comma/whitespace sensor vectors → fixed-width
//!   rows; short lines are zero-filled, long lines are truncated — the
//!   fabric width is the contract, the sensor adapts.
//!
//! Non-blocking discipline: ingest never blocks on the fabric lock beyond
//! the append itself, and malformed lines are skipped with a count (bad
//! telemetry is data about the sensor, not an error to die on).

use std::io;

use crate::fabric::Fabric;

/// Depth-below-transducer ingest (NMEA DBT). Single output column: meters.
pub struct DbtIngest;

impl DbtIngest {
    pub const COLS: u64 = 1;

    /// Parse one `$--DBT,x.x,f,y.y,F,z.z,M*hh` sentence → meters.
    /// Returns None for non-DBT sentences or missing meter field.
    pub fn parse(line: &str) -> Option<f32> {
        let body = line.strip_prefix('$')?;
        let mut fields = body.split(',');
        let kind = fields.next()?.strip_suffix("DBT")?;
        let _ = kind; // talker id, any of GP/SD/II/... is fine
        let _feet = fields.next()?;
        let _f = fields.next()?;
        let _fathoms = fields.next()?;
        let _f = fields.next()?;
        let meters = fields.next()?;
        let meters = meters.split('*').next()? // drop NMEA checksum
            .trim_end_matches(|c: char| c.is_alphabetic());
        meters.parse::<f32>().ok()
    }
}

/// Generic fixed-width CSV/whitespace sensor vector layout.
pub struct CsvLayout {
    pub width: u64,
}

impl CsvLayout {
    /// Ingest one line into the fabric. Missing fields zero-fill; extra
    /// fields truncate. Returns the row index.
    pub fn ingest(&self, fabric: &mut Fabric, line: &str) -> io::Result<Option<u64>> {
        let mut v = vec![0.0f32; self.width as usize];
        let mut used = 0usize;
        for tok in line.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()) {
            match tok.parse::<f32>() {
                Ok(x) if used < v.len() => {
                    v[used] = x;
                    used += 1;
                }
                Ok(_) => break, // truncated — width is the contract
                Err(_) => continue, // skip non-numeric junk
            }
        }
        if used == 0 {
            return Ok(None); // nothing numeric on this line
        }
        fabric.append(&v).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dbt_parses_meters() {
        let line = "$SDDBT,17.0,f,2.8,F,5.2,M*3B";
        let m = DbtIngest::parse(line).unwrap();
        assert!((m - 5.2).abs() < 1e-4);
        assert_eq!(DbtIngest::parse("$GPGGA,blah"), None);
    }

    #[test]
    fn csv_layout_width_is_contract() {
        let dir = std::env::temp_dir().join(format!(
            "eos-tel-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = Fabric::create(dir.join("t.fab"), 4).unwrap();
        let lay = CsvLayout { width: 4 };
        let r = lay.ingest(&mut f, "1.5, -2, 3, 4, 5, 6").unwrap().unwrap();
        assert_eq!(f.row(r), &[1.5, -2.0, 3.0, 4.0]); // truncated to width
        let r = lay.ingest(&mut f, "9 8").unwrap().unwrap();
        assert_eq!(f.row(r), &[9.0, 8.0, 0.0, 0.0]); // zero-filled
        assert_eq!(lay.ingest(&mut f, "junk only").unwrap(), None);
    }
}
