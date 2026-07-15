//! NVIDIA GPU backend via NVML. Default on Linux and Windows.
//!
//! `nvml-wrapper` loads the NVML library at runtime (`libnvidia-ml.so` on Linux,
//! `nvml.dll` on Windows), so this compiles on any such host; `try_new` returns
//! `None` when no NVIDIA driver/device is present and `default_gpu_collector`
//! falls back to the mock (or the DXGI backend on Windows).

use crate::collect::gpu::{GpuCollector, GpuSample};
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::enums::device::UsedGpuMemory;

pub struct NvmlGpu {
    nvml: Nvml,
}

impl NvmlGpu {
    /// Initialize NVML, returning `None` if unavailable on this host.
    pub fn try_new() -> Option<Self> {
        Nvml::init().ok().map(|nvml| Self { nvml })
    }
}

impl GpuCollector for NvmlGpu {
    fn sample(&mut self) -> Vec<GpuSample> {
        let count = match self.nvml.device_count() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        for i in 0..count {
            let Ok(dev) = self.nvml.device_by_index(i) else {
                continue;
            };
            let util = dev.utilization_rates().map(|u| u.gpu as f32).unwrap_or(0.0);
            let (used, total) = dev
                .memory_info()
                .map(|m| (m.used, m.total))
                .unwrap_or((0, 0));
            let temp = dev.temperature(TemperatureSensor::Gpu).unwrap_or(0);
            let power = dev.power_usage().map(|mw| mw / 1000).unwrap_or(0);
            let per_pid = dev
                .running_compute_processes()
                .unwrap_or_default()
                .into_iter()
                .map(|p| {
                    let vram = match p.used_gpu_memory {
                        UsedGpuMemory::Used(b) => b,
                        _ => 0,
                    };
                    (p.pid, vram, util)
                })
                .collect();
            out.push(GpuSample {
                index: i,
                util_pct: util,
                mem_used: used,
                mem_total: total,
                temp_c: temp,
                power_w: power,
                per_pid,
            });
        }
        out
    }

    fn name(&self) -> &str {
        "nvml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvml_samples_are_shaped_or_absent() {
        match NvmlGpu::try_new() {
            Some(mut g) => {
                for s in g.sample() {
                    assert!(s.mem_used <= s.mem_total);
                }
            }
            None => { /* No NVIDIA device here; hardware validation deferred. */ }
        }
    }
}
