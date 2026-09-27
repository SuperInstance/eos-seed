//! fabric.rs — memory-mapped continuous row vector matrix.
//!
//! A file-backed, memory-mapped append-only array of f32 rows.
//! Layout: 64-byte aligned header, then rows × cols × 4 bytes of raw f32.
//!
//! Paradigm (quilt-dba): the file IS the database. Appends are sequential,
//! zero-copy (memcpy straight into the map), and the header is the only
//! metadata. Readers map the file and never block writers.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

use memmap2::MmapMut;

/// 64-byte aligned binary header.
///
/// bytes 0..8   magic  b"EOSFAB01"
/// bytes 8..16  rows   u64 LE — total rows appended
/// bytes 16..24 cols   u64 LE — vector width (fixed at create)
/// bytes 24..64 reserved for future tissue metadata
pub const HEADER_LEN: u64 = 64;
const MAGIC: &[u8; 8] = b"EOSFAB01";

pub struct Fabric {
    file: File,
    map: MmapMut,
    cols: u64,
    rows: u64,
}

fn parse_header(bytes: &[u8]) -> io::Result<(u64, u64)> {
    if bytes.len() < HEADER_LEN as usize {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file shorter than header"));
    }
    if &bytes[0..8] != MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad magic — not an EOS fabric"));
    }
    let rows = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let cols = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    if cols == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "cols must be > 0"));
    }
    Ok((rows, cols))
}

impl Fabric {
    /// Create a fresh fabric file with `cols`-wide rows.
    pub fn create<P: AsRef<Path>>(path: P, cols: u64) -> io::Result<Fabric> {
        let mut file = OpenOptions::new()
            .read(true).write(true).create_new(true)
            .open(path)?;
        let header = Self::make_header(0, cols);
        file.set_len(HEADER_LEN)?;
        use std::io::{Seek, Write};
        file.seek(std::io::SeekFrom::Start(0))?;
        file.write_all(&header)?;
        file.sync_all()?;
        let map = unsafe { MmapMut::map_mut(&file)? };
        Ok(Fabric { file, map, cols, rows: 0 })
    }

    /// Open an existing fabric. Reads dims from the header on disk.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Fabric> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let map = unsafe { MmapMut::map_mut(&file)? };
        let (rows, cols) = parse_header(&map[..HEADER_LEN as usize])?;
        Ok(Fabric { file, map, cols, rows })
    }

    fn make_header(rows: u64, cols: u64) -> [u8; HEADER_LEN as usize] {
        let mut h = [0u8; HEADER_LEN as usize];
        h[0..8].copy_from_slice(MAGIC);
        h[8..16].copy_from_slice(&rows.to_le_bytes());
        h[16..24].copy_from_slice(&cols.to_le_bytes());
        h
    }

    pub fn cols(&self) -> u64 { self.cols }
    pub fn rows(&self) -> u64 { self.rows }

    /// Append one row. Zero-copy: the slice is copied straight into the
    /// memory map; no intermediate buffers. Returns the new row's index.
    pub fn append(&mut self, vector: &[f32]) -> io::Result<u64> {
        assert_eq!(vector.len() as u64, self.cols, "vector width must match fabric cols");
        let row_idx = self.rows;
        let byte_off = HEADER_LEN + row_idx * self.cols * 4;
        let new_len = byte_off + self.cols * 4;
        if (self.map.len() as u64) < new_len {
            self.file.set_len(new_len)?;
            // SAFETY: file length was just extended; remap covers it.
            self.map = unsafe { MmapMut::map_mut(&self.file)? };
        }
        let off = byte_off as usize;
        let bytes: &[u8] = bytemuck_slice(vector);
        self.map[off..off + bytes.len()].copy_from_slice(bytes);
        self.rows += 1;
        self.write_header()?;
        Ok(row_idx)
    }

    /// Immutable view of row `idx`. Panics if out of range.
    pub fn row(&self, idx: u64) -> &[f32] {
        assert!(idx < self.rows, "row {idx} out of range (rows={})", self.rows);
        let off = (HEADER_LEN + idx * self.cols * 4) as usize;
        let len = (self.cols * 4) as usize;
        bytemuck_slice_inv(&self.map[off..off + len])
    }

    fn write_header(&mut self) -> io::Result<()> {
        let h = Self::make_header(self.rows, self.cols);
        self.map[..HEADER_LEN as usize].copy_from_slice(&h);
        self.map.flush()
    }

    /// Durability point: flush data pages to disk.
    pub fn sync(&self) -> io::Result<()> {
        self.map.flush()?;
        self.file.sync_all()
    }
}

// Minimal f32-slice <-> u8-slice reinterpretation without external crates.
// SAFETY contracts satisfied: f32 and u8 alignment (4 >= 1) and size exact.
fn bytemuck_slice(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}
fn bytemuck_slice_inv(b: &[u8]) -> &[f32] {
    unsafe { std::slice::from_raw_parts(b.as_ptr() as *const f32, b.len() / 4) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("eos-test-{name}-{}-{}.fab",
                       std::process::id(),
                       std::time::SystemTime::now()
                           .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        p
    }

    #[test]
    fn append_and_read_roundtrip() {
        let path = tmp("roundtrip");
        let mut f = Fabric::create(&path, 4).unwrap();
        assert_eq!(f.append(&[1.0, -2.0, 3.5, 0.0]).unwrap(), 0);
        assert_eq!(f.append(&[-9.0, 8.0, 7.0, -6.0]).unwrap(), 1);
        assert_eq!(f.rows(), 2);
        assert_eq!(f.row(0), &[1.0, -2.0, 3.5, 0.0]);
        assert_eq!(f.row(1), &[-9.0, 8.0, 7.0, -6.0]);
        f.sync().unwrap();
        drop(f);
        // reopen: header on disk must restore dims
        let f = Fabric::open(&path).unwrap();
        assert_eq!(f.rows(), 2);
        assert_eq!(f.cols(), 4);
        assert_eq!(f.row(1), &[-9.0, 8.0, 7.0, -6.0]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    #[should_panic(expected = "vector width must match")]
    fn width_mismatch_panics() {
        let path = tmp("mismatch");
        let mut f = Fabric::create(&path, 3).unwrap();
        let _ = f.append(&[1.0, 2.0]);
    }
}
