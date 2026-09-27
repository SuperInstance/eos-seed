//! Shared-memory serialization stream: the fabric broadcast lane.
//!
//! The core loop publishes the latest frame + gate state into a mmap'd
//! file; an external skin (WebGPU canvas, UE5 dumb-terminal, anything)
//! maps the same file read-only and renders. Zero sockets, zero copies
//! past the one publish write. Fixed-size, versioned, sequence-counted.

use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::io;
use std::path::Path;

pub const SHM_MAGIC: [u8; 8] = *b"EOSSHMP1";

/// Fixed layout. `max_dims` caps the frame payload so external readers can
/// map a known size; readers validate `dims <= max_dims` and `seq` for liveness.
#[repr(C, align(64))]
pub struct ShmPayload {
    pub magic: [u8; 8],
    pub seq: u64,       // bumped every publish; 0 = nothing published yet
    pub dims: u32,      // vector width of the published frame
    pub rows: u64,      // total rows appended to the source fabric so far
    pub frame: [f32; 256],   // latest 16x16 state snapshot
    pub gate: [u8; 64],      // latest 256 gate cells, 2 bits each (packed)
    pub object_x: u32,       // tracked object position (kernel telemetry)
    pub object_y: u32,
}

pub struct ShmBroadcastStream {
    #[allow(dead_code)] // keeps the mapping alive; access goes through ptr
    mmap: MmapMut,
    ptr: *mut ShmPayload,
}
// Safety: the raw pointer aliases only this instance's private mmap; all
// access is through &mut self on the owning thread.
unsafe impl Send for ShmBroadcastStream {}

impl ShmBroadcastStream {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).create(true).open(path)?;
        let size = std::mem::size_of::<ShmPayload>();
        if (file.metadata()?.len() as usize) < size {
            file.set_len(size as u64)?;
        }
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        let p = mmap.as_mut_ptr() as *mut ShmPayload;
        unsafe {
            if (*p).magic != SHM_MAGIC {
                std::ptr::write_bytes(p, 0, 1);
                (*p).magic = SHM_MAGIC;
                mmap.flush()?;
            }
        }
        let ptr = mmap.as_mut_ptr() as *mut ShmPayload;
        Ok(Self { mmap, ptr })
    }

    #[inline]
    fn ptr(&self) -> *mut ShmPayload {
        self.ptr
    }

    /// Publish the latest state. seq is bumped first-write so an external
    /// skin polling `seq` sees liveness monotonically.
    pub fn publish(
        &mut self,
        frame: &[f32],
        gate_packed: &[u8],
        rows: u64,
        object: (u32, u32),
    ) -> io::Result<u64> {
        if frame.len() > 256 || gate_packed.len() > 64 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "payload exceeds shm layout"));
        }
        let p = self.ptr();
        unsafe {
            (*p).seq += 1;
            (*p).dims = frame.len() as u32;
            (*p).rows = rows;
            (*p).object_x = object.0;
            (*p).object_y = object.1;
            std::ptr::copy_nonoverlapping(frame.as_ptr(), (*p).frame.as_mut_ptr(), frame.len());
            std::ptr::copy_nonoverlapping(gate_packed.as_ptr(), (*p).gate.as_mut_ptr(), gate_packed.len());
            Ok((*p).seq)
        }
    }

    pub fn seq(&self) -> u64 { unsafe { (*self.ptr()).seq } }
}

/// The external-skin side: read-only mapping over the same file.
pub struct ShmReader {
    map: memmap2::Mmap,
}

impl ShmReader {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        Ok(Self { map: unsafe { MmapOptions::new().map(&file)? } })
    }
    pub fn payload(&self) -> &ShmPayload { unsafe { &*(self.map.as_ptr() as *const ShmPayload) } }
    pub fn seq(&self) -> u64 { self.payload().seq }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_roundtrip() {
        let dir = std::env::temp_dir().join(format!("eos-shm-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("broadcast.shm");

        let mut stream = ShmBroadcastStream::open(&path).expect("open");
        assert_eq!(stream.seq(), 0, "fresh lane: nothing published");

        let frame = vec![0.5f32; 256];
        let gate = vec![0b01_01_01_01u8; 64]; // all +1
        let s1 = stream.publish(&frame, &gate, 42, (7, 9)).unwrap();
        assert_eq!(s1, 1);

        // external skin maps the same file independently
        let reader = ShmReader::open(&path).unwrap();
        let p = reader.payload();
        assert_eq!(p.magic, SHM_MAGIC);
        assert_eq!(p.seq, 1);
        assert_eq!(p.rows, 42);
        assert_eq!(p.object_x, 7);
        assert_eq!(p.frame[0], 0.5);
        assert_eq!(p.frame[255], 0.5);
        assert_eq!(p.gate[0], 0b01_01_01_01);

        // second publish bumps liveness
        stream.publish(&vec![-1.0; 256], &gate, 43, (8, 9)).unwrap();
        assert_eq!(reader.seq(), 2);
        assert_eq!(reader.payload().frame[0], -1.0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
