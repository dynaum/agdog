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

/// Backend used when no real GPU is readable: reports nothing at all.
///
/// This is deliberately not the mock. A host with an AMD/Intel GPU, or an
/// NVIDIA card whose driver failed to load, must show an empty GPU panel
/// rather than fabricated devices that look like working hardware.
pub struct NoGpu;

impl GpuCollector for NoGpu {
    fn sample(&mut self) -> Vec<GpuSample> {
        Vec::new()
    }

    fn name(&self) -> &str {
        "none"
    }
}

/// Return the best available GPU backend for this build and host.
///
/// Order: demo mock (opt-in), then the platform's real backend. When no real
/// backend initializes we return `NoGpu`, never the mock.
pub fn default_gpu_collector() -> Box<dyn GpuCollector> {
    // Demo/screenshot mode forces the mock GPU on any platform.
    if crate::demo::enabled() {
        return Box::new(MockGpu::new());
    }
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
    Box::new(NoGpu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_gpu_reports_no_devices() {
        let mut g = NoGpu;
        assert!(g.sample().is_empty());
        assert_eq!(g.name(), "none");
    }

    #[test]
    fn default_backend_is_never_the_mock_outside_demo_mode() {
        // Guards the core honesty property: fabricated GPU data may only ever
        // appear when the user explicitly opted into demo mode.
        if !crate::demo::enabled() {
            assert_ne!(default_gpu_collector().name(), "mock");
        }
    }
}
