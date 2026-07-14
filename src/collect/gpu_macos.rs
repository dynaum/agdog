//! Apple Silicon GPU backend. Default on macOS, no feature flag, no sudo.
//!
//! GPU utilization comes from the `IOAccelerator` `PerformanceStatistics`
//! dictionary via `ioreg`, which is readable without root. Unified-memory
//! totals come from `sysinfo`. macOS does not expose per-process VRAM, so
//! `per_pid` is always empty and attribution groups by process CPU/memory.

use crate::collect::gpu::{GpuCollector, GpuSample};
use sysinfo::System;

/// Extract "Device Utilization %" from `ioreg -c IOAccelerator` output.
/// Pure and sudo-free so it can be unit-tested against captured output.
pub fn parse_ioreg_util(out: &str) -> Option<f32> {
    let key = "\"Device Utilization %\"=";
    let start = out.find(key)? + key.len();
    let num: String = out[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num.parse().ok()
}

/// Apple Silicon unified-memory + GPU collector.
pub struct MacGpu {
    sys: System,
}

impl MacGpu {
    /// Always available on macOS.
    pub fn try_new() -> Option<Self> {
        Some(Self { sys: System::new() })
    }

    fn run_ioreg() -> Option<String> {
        std::process::Command::new("ioreg")
            .args(["-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    }
}

impl GpuCollector for MacGpu {
    fn sample(&mut self) -> Vec<GpuSample> {
        let util = Self::run_ioreg()
            .as_deref()
            .and_then(parse_ioreg_util)
            .unwrap_or(0.0);
        self.sys.refresh_memory();
        vec![GpuSample {
            index: 0,
            util_pct: util,
            mem_used: self.sys.used_memory(),
            mem_total: self.sys.total_memory(),
            temp_c: 0,
            power_w: 0,
            per_pid: Vec::new(),
        }]
    }

    fn name(&self) -> &str {
        "macos"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_device_utilization() {
        let s = r#""Renderer Utilization %"=1,"Device Utilization %"=42,"SplitSceneCount"=0"#;
        assert_eq!(parse_ioreg_util(s), Some(42.0));
    }

    #[test]
    fn missing_key_returns_none() {
        assert_eq!(parse_ioreg_util("no stats here"), None);
    }

    #[test]
    fn live_backend_reports_sane_values() {
        // Exercises real ioreg on the macOS build (no sudo needed).
        if let Some(mut g) = MacGpu::try_new() {
            let s = g.sample();
            assert_eq!(s.len(), 1);
            assert!((0.0..=100.0).contains(&s[0].util_pct));
            assert!(s[0].mem_total > 0);
        }
    }
}
