//! Apple Silicon GPU backend (feature `macos`).
//!
//! GPU utilization and power come from `powermetrics` (needs root; degrades to
//! zeros without it). Unified-memory totals come from `sysinfo`. macOS does not
//! expose per-process VRAM, so `per_pid` is always empty and attribution groups
//! by process CPU/memory instead.

use crate::collect::gpu::{GpuCollector, GpuSample};
use sysinfo::System;

/// Parse `powermetrics --samplers gpu_power` output into a single GPU sample.
/// Pure and sudo-free so it can be unit-tested against captured output.
pub fn parse_powermetrics(out: &str) -> Vec<GpuSample> {
    let mut util = 0.0f32;
    let mut power_w = 0u32;
    for line in out.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("GPU HW active residency:") {
            util = leading_f32(rest);
        } else if let Some(rest) = l.strip_prefix("GPU Power:") {
            power_w = leading_u32(rest) / 1000; // mW -> W
        }
    }
    vec![GpuSample {
        index: 0,
        util_pct: util,
        mem_used: 0,
        mem_total: 0,
        temp_c: 0,
        power_w,
        per_pid: Vec::new(),
    }]
}

fn leading_f32(s: &str) -> f32 {
    let s = s.trim();
    let num: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num.parse().unwrap_or(0.0)
}

fn leading_u32(s: &str) -> u32 {
    let s = s.trim();
    let num: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().unwrap_or(0)
}

/// Apple Silicon unified-memory + GPU collector.
pub struct MacGpu {
    sys: System,
}

impl MacGpu {
    /// Always available on macOS; GPU util is best-effort (needs root).
    pub fn try_new() -> Option<Self> {
        Some(Self { sys: System::new() })
    }

    fn run_powermetrics() -> Option<String> {
        std::process::Command::new("powermetrics")
            .args(["--samplers", "gpu_power", "-n1", "-i", "200"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    }
}

impl GpuCollector for MacGpu {
    fn sample(&mut self) -> Vec<GpuSample> {
        let mut samples = match Self::run_powermetrics() {
            Some(out) => parse_powermetrics(&out),
            None => parse_powermetrics(""),
        };
        self.sys.refresh_memory();
        let total = self.sys.total_memory();
        let used = self.sys.used_memory();
        for s in samples.iter_mut() {
            s.mem_total = total;
            s.mem_used = used;
        }
        samples
    }

    fn name(&self) -> &str {
        "macos"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
**** GPU usage ****

GPU HW active frequency: 388 MHz
GPU HW active residency:  45.20% (444 MHz: 12%)
GPU idle residency:  54.80%
GPU Power: 1234 mW
";

    #[test]
    fn parses_util_and_power() {
        let s = parse_powermetrics(SAMPLE);
        assert_eq!(s.len(), 1);
        assert!((s[0].util_pct - 45.20).abs() < 0.01);
        assert_eq!(s[0].power_w, 1);
    }

    #[test]
    fn empty_input_yields_zeros() {
        let s = parse_powermetrics("");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].util_pct, 0.0);
    }
}
