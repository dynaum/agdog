//! sysinfo-based process, CPU, and RAM collector.

use crate::model::ResourceSample;
use sysinfo::{
    CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System,
};

/// Collects real process and system metrics via `sysinfo`.
pub struct SystemCollector {
    sys: System,
}

impl SystemCollector {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_processes(ProcessRefreshKind::everything())
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_processes(ProcessesToUpdate::All, true);
        Self { sys }
    }

    /// Refresh and return one `ResourceSample` per live process.
    /// GPU fields are left zero here; the GPU backend fills them by pid.
    pub fn sample(&mut self) -> Vec<ResourceSample> {
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys
            .processes()
            .values()
            .map(|p| {
                let parts: Vec<String> = p
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect();
                let cmd = if parts.is_empty() {
                    p.name().to_string_lossy().into_owned()
                } else {
                    parts.join(" ")
                };
                ResourceSample {
                    pid: p.pid().as_u32(),
                    ppid: p.parent().map(|x| x.as_u32()).unwrap_or(0),
                    cpu_pct: p.cpu_usage(),
                    rss_bytes: p.memory(),
                    gpu_pct: 0.0,
                    vram_bytes: 0,
                    gpu_index: None,
                    cmd,
                }
            })
            .collect()
    }

    /// System-wide CPU utilization (0-100).
    pub fn total_cpu_pct(&self) -> f32 {
        self.sys.global_cpu_usage()
    }

    /// (used_bytes, total_bytes) of physical memory.
    pub fn total_mem(&self) -> (u64, u64) {
        (self.sys.used_memory(), self.sys.total_memory())
    }
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_includes_current_process() {
        let mut c = SystemCollector::new();
        let samples = c.sample();
        assert!(!samples.is_empty(), "expected at least one process");
        let me = std::process::id();
        assert!(
            samples.iter().any(|s| s.pid == me),
            "current pid {me} not found in samples"
        );
    }

    #[test]
    fn total_mem_is_sane() {
        let mut c = SystemCollector::new();
        let _ = c.sample();
        let (used, total) = c.total_mem();
        assert!(total > 0);
        assert!(used <= total);
    }
}
