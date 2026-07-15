//! GPU collector trait and shared GpuSample type.

use crate::collect::gpu_mock::MockGpu;

/// A per-device GPU observation for one tick.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuSample {
    pub index: u32,
    pub util_pct: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub temp_c: u32,
    pub power_w: u32,
    /// (pid, vram_bytes, util_pct) for processes using this device.
    pub per_pid: Vec<(u32, u64, f32)>,
}

/// A pluggable GPU metrics backend.
pub trait GpuCollector {
    fn sample(&mut self) -> Vec<GpuSample>;
    fn name(&self) -> &str;
}

/// Return the best available GPU backend for this build and host.
///
/// Real backends (NVML, macOS) are wired in behind feature flags in later
/// tasks; until one initializes successfully we always fall back to the mock,
/// so the binary runs on any machine.
pub fn default_gpu_collector() -> Box<dyn GpuCollector> {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        if let Some(nvml) = crate::collect::gpu_nvml::NvmlGpu::try_new() {
            return Box::new(nvml);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(mac) = crate::collect::gpu_macos::MacGpu::try_new() {
            return Box::new(mac);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(win) = crate::collect::gpu_windows::WindowsGpu::try_new() {
            return Box::new(win);
        }
    }
    Box::new(MockGpu::new())
}
