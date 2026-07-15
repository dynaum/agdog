//! Collectors: assemble ResourceSamples from system and GPU backends.

pub mod gpu;
pub mod gpu_mock;
pub mod system;

// GPU backends are chosen by target OS at build time, so the default binary
// uses the right one for the host with no feature flags.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub mod gpu_nvml;

#[cfg(target_os = "macos")]
pub mod gpu_macos;

#[cfg(target_os = "windows")]
pub mod gpu_windows;
