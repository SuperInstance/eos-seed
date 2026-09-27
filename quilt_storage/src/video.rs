//! Asynchronous video frame-buffer tokenizer.
//!
//! Down-samples raw packed RGB frames into BT.601 luma vectors at fabric
//! width and appends them on a background thread. The fabric moves INTO
//! the worker; on shutdown it comes back out through a bounded channel —
//! the thread boundary is message-passing all the way, no shared mutable
//! state, no unsafe.

use crate::fabric::Fabric;
use crossbeam_channel::{bounded, Sender};
use std::io;
use std::thread::JoinHandle;

pub enum FrameCommand {
    /// Raw packed RGB bytes (len = w*h*3), downsampled to fabric width.
    Frame { width: usize, height: usize, rgb: Vec<u8> },
    Shutdown,
}

pub struct AsyncVideoTokenizer {
    tx: Sender<FrameCommand>,
    handle: Option<JoinHandle<io::Result<Fabric>>>,
}

impl AsyncVideoTokenizer {
    /// Spin up the tokenizer worker. The fabric is owned by the worker
    /// until `shutdown()` hands it back (synced, all frames committed).
    pub fn spin_up(fabric: Fabric, queue_depth: usize) -> Self {
        let (tx, rx) = bounded::<FrameCommand>(queue_depth);
        let (back_tx, _back_rx) = bounded::<bool>(1);
        let handle = std::thread::spawn(move || {
            let mut fabric = fabric;
            loop {
                match rx.recv() {
                    Ok(FrameCommand::Frame { width, height, rgb }) => {
                        let n = width * height;
                        if rgb.len() != n * 3 {
                            continue; // malformed frame: skip it, lane stays alive
                        }
                        let mut v = vec![0.0f32; fabric.cols() as usize];
                        let take = n.min(v.len());
                        for i in 0..take {
                            let (r, g, b) = (rgb[i * 3] as f32, rgb[i * 3 + 1] as f32, rgb[i * 3 + 2] as f32);
                            v[i] = 0.299 * r + 0.587 * g + 0.114 * b; // BT.601 luma
                        }
                        if let Err(e) = fabric.append(&v) { break; } else { continue; }
                    }
                    Ok(FrameCommand::Shutdown) | Err(_) => break,
                }
            }
            let out = fabric.sync().map(|_| fabric);
            let _ = back_tx.send(out.is_ok()); // liveness ping, no move
            out
        });
        AsyncVideoTokenizer { tx, handle: Some(handle) }
    }

    #[inline]
    pub fn submit_frame(&self, width: usize, height: usize, rgb: Vec<u8>) -> bool {
        self.tx.send(FrameCommand::Frame { width, height, rgb }).is_ok()
    }

    /// Drain, sync, and reclaim the fabric. Blocks until the worker stops.
    pub fn shutdown(mut self) -> io::Result<Fabric> {
        let _ = self.tx.send(FrameCommand::Shutdown);
        drop(self.tx); // release the back-channel clone
        self.handle.take().expect("worker handle").join().expect("worker panicked")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_to_luma_ingest_pipeline() {
        let dir = std::env::temp_dir().join(format!("eos-vid-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("frames.fab");

        let (w, h) = (16usize, 16usize);
        let fabric = Fabric::create(&path, (w * h) as u64).unwrap();
        let tokenizer = AsyncVideoTokenizer::spin_up(fabric, 8);

        // three frames brightening uniformly: frame k = grey level k*50
        for k in 0..3u8 {
            let frame = vec![k * 50; w * h * 3];
            assert!(tokenizer.submit_frame(w, h, frame));
        }
        let fabric = tokenizer.shutdown().unwrap();
        assert_eq!(fabric.rows(), 3);
        let f0 = fabric.row(0);
        assert_eq!(f0[0], 0.0);          // 0.299*0 + 0.587*0 + 0.114*0
        let f1 = fabric.row(1);
        assert!((f1[0] - 50.0).abs() < 1e-4); // grey 50 -> luma 50
        let f2 = fabric.row(2);
        assert!((f2[255] - 100.0).abs() < 1e-4);

        // malformed frame skipped without killing the lane
        let tokenizer = AsyncVideoTokenizer::spin_up(fabric, 8);
        assert!(tokenizer.submit_frame(w, h, vec![0; 10]));
        assert!(tokenizer.submit_frame(w, h, vec![200; w * h * 3]));
        let fabric = tokenizer.shutdown().unwrap();
        assert_eq!(fabric.rows(), 4);
        assert!((fabric.row(3)[0] - 200.0).abs() < 1e-4);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
