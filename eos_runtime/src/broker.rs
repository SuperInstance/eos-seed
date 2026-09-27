//! broker.rs — system-agnostic hardware capability detector.
//!
//! Reads what the host actually offers (cores, memory, architecture) so
//! the runtime can size its worker ring without assumptions. No external
//! sysinfo crates — /proc when present, std otherwise.

use std::thread::available_parallelism;

#[derive(Debug, Clone)]
pub struct Capabilities {
    pub arch: &'static str,
    pub logical_cores: usize,
    pub total_mem_kib: Option<u64>,
}

impl Capabilities {
    pub fn detect() -> Capabilities {
        let total_mem_kib = std::fs::read_to_string("/proc/meminfo").ok().and_then(|s| {
            s.lines().next().and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        });
        Capabilities {
            arch: std::env::consts::ARCH,
            logical_cores: available_parallelism().map(|n| n.get()).unwrap_or(1),
            total_mem_kib,
        }
    }

    /// Worker-ring size: use most cores, leave one for the broker thread,
    /// cap at 8 for the seed (deep searches will want more later).
    pub fn ring_size(&self) -> usize {
        (self.logical_cores.saturating_sub(1)).clamp(1, 8)
    }
}

/// Best-effort core pinning for worker `idx`. Never fatal: a busy desktop
/// or a container without affinity support just runs unpinned.
pub fn pin_worker(idx: usize) -> bool {
    let cores = core_affinity::get_core_ids().unwrap_or_default();
    if cores.is_empty() {
        return false;
    }
    core_affinity::set_for_current(cores[idx % cores.len()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_is_sane() {
        let caps = Capabilities::detect();
        assert!(caps.logical_cores >= 1);
        assert!(caps.ring_size() >= 1 && caps.ring_size() <= 8);
        if cfg!(target_os = "linux") {
            assert!(caps.total_mem_kib.is_some());
        }
    }
}
