//! Deterministic mock GPU backend (default; always available).

use crate::collect::gpu::{GpuCollector, GpuSample};

const GB: u64 = 1024 * 1024 * 1024;
const MEM_TOTAL: u64 = 24 * GB;

/// A deterministic fake GPU backend that varies across ticks without `rand`.
pub struct MockGpu {
    counter: u64,
}

impl MockGpu {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}

impl Default for MockGpu {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuCollector for MockGpu {
    fn sample(&mut self) -> Vec<GpuSample> {
        self.counter = self.counter.wrapping_add(1);
        (0..4u32)
            .map(|i| {
                let mix = self
                    .counter
                    .wrapping_add(i as u64 * 7)
                    .wrapping_mul(2654435761);
                let util = (mix % 100) as f32;
                let mem_used = (mix >> 8) % 24 * GB;
                let temp_c = 40 + ((mix >> 16) % 45) as u32;
                let power_w = 100 + ((mix >> 24) % 600) as u32;
                GpuSample {
                    index: i,
                    util_pct: util,
                    mem_used,
                    mem_total: MEM_TOTAL,
                    temp_c,
                    power_w,
                    per_pid: Vec::new(),
                }
            })
            .collect()
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_four_gpus_with_valid_memory() {
        let mut gpu = MockGpu::new();
        let samples = gpu.sample();
        assert_eq!(samples.len(), 4);
        for s in &samples {
            assert!(s.mem_used <= s.mem_total, "mem_used exceeds mem_total");
            assert!((0.0..=100.0).contains(&s.util_pct));
        }
    }

    #[test]
    fn values_change_across_ticks() {
        let mut gpu = MockGpu::new();
        let first = gpu.sample();
        let second = gpu.sample();
        assert_ne!(first, second, "mock GPU values should vary across ticks");
    }
}
