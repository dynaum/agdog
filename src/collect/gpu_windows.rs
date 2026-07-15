//! Windows GPU backend. Default on Windows when NVML is unavailable.
//!
//! NVIDIA GPUs are handled by the NVML backend (preferred, wired in `gpu.rs`).
//! This DXGI backend covers any adapter (Intel, AMD, NVIDIA): per-adapter VRAM
//! via `IDXGIAdapter3::QueryVideoMemoryInfo`, and overall GPU utilization via the
//! PDH `\GPU Engine(*)\Utilization Percentage` counters. No sudo, no external
//! tools. Falls back to zeros on any error.

use crate::collect::gpu::{GpuCollector, GpuSample};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ADAPTER_DESC1, DXGI_ADAPTER_FLAG, DXGI_ADAPTER_FLAG_SOFTWARE,
    DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO, IDXGIAdapter1, IDXGIAdapter3,
    IDXGIFactory1,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW,
    PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::core::{Interface, PCWSTR};

/// DXGI + PDH GPU collector for Windows (non-NVIDIA or NVML-unavailable hosts).
pub struct WindowsGpu;

impl WindowsGpu {
    /// Available whenever at least one hardware adapter is enumerable.
    pub fn try_new() -> Option<Self> {
        if dxgi_adapters().is_empty() {
            None
        } else {
            Some(WindowsGpu)
        }
    }
}

impl GpuCollector for WindowsGpu {
    fn sample(&mut self) -> Vec<GpuSample> {
        // One overall utilization figure; applied to each adapter (fine for the
        // common single-GPU case).
        let util = pdh_gpu_util();
        dxgi_adapters()
            .into_iter()
            .enumerate()
            .map(|(i, (_name, used, total))| GpuSample {
                index: i as u32,
                util_pct: util,
                mem_used: used,
                mem_total: total,
                temp_c: 0,
                power_w: 0,
                per_pid: Vec::new(),
            })
            .collect()
    }

    fn name(&self) -> &str {
        "windows-dxgi"
    }
}

/// Enumerate GPU adapters: (name, vram_used_bytes, vram_total_bytes).
/// Returns an empty vec on any failure; never panics.
fn dxgi_adapters() -> Vec<(String, u64, u64)> {
    let mut out = Vec::new();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
            Ok(f) => f,
            Err(_) => return out,
        };

        let mut index: u32 = 0;
        loop {
            let adapter1: IDXGIAdapter1 = match factory.EnumAdapters1(index) {
                Ok(a) => a,
                Err(_) => break,
            };
            index += 1;

            let mut desc = DXGI_ADAPTER_DESC1::default();
            if adapter1.GetDesc1(&mut desc).is_err() {
                continue;
            }

            // Skip the Microsoft Basic Render (software) adapter.
            if (DXGI_ADAPTER_FLAG(desc.Flags as i32).0 & DXGI_ADAPTER_FLAG_SOFTWARE.0) != 0 {
                continue;
            }

            let name = {
                let end = desc
                    .Description
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(desc.Description.len());
                String::from_utf16_lossy(&desc.Description[..end])
            };

            let mut used: u64 = 0;
            let mut total: u64 = desc.DedicatedVideoMemory as u64;
            if let Ok(adapter3) = adapter1.cast::<IDXGIAdapter3>() {
                let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                if adapter3
                    .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info)
                    .is_ok()
                {
                    used = info.CurrentUsage;
                    if total == 0 {
                        total = info.Budget;
                    }
                }
            }

            out.push((name, used, total));
        }
    }
    out
}

/// Overall GPU utilization percent (0..100) via PDH engine counters.
fn pdh_gpu_util() -> f32 {
    unsafe {
        let mut query = PDH_HQUERY::default();
        if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
            return 0.0;
        }
        let result = collect_gpu_util(query);
        let _ = PdhCloseQuery(query);
        result.unwrap_or(0.0)
    }
}

unsafe fn collect_gpu_util(query: PDH_HQUERY) -> Option<f32> {
    unsafe {
        let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage\0"
            .encode_utf16()
            .collect();

        let mut counter = PDH_HCOUNTER::default();
        if PdhAddEnglishCounterW(query, PCWSTR(path.as_ptr()), 0, &mut counter) != 0 {
            return None;
        }

        if PdhCollectQueryData(query) != 0 {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        if PdhCollectQueryData(query) != 0 {
            return None;
        }

        let mut buf_size: u32 = 0;
        let mut item_count: u32 = 0;
        let _ = PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &mut buf_size,
            &mut item_count,
            None,
        );
        if buf_size == 0 || item_count == 0 {
            return None;
        }

        let mut bytes = vec![0u8; buf_size as usize];
        let items_ptr = bytes.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;

        if PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &mut buf_size,
            &mut item_count,
            Some(items_ptr),
        ) != 0
        {
            return None;
        }

        let items = std::slice::from_raw_parts(items_ptr, item_count as usize);
        let mut sum = 0.0f64;
        for item in items {
            if item.FmtValue.CStatus == 0 {
                let v = item.FmtValue.Anonymous.doubleValue;
                if v.is_finite() && v > 0.0 {
                    sum += v;
                }
            }
        }
        Some(sum.clamp(0.0, 100.0) as f32)
    }
}
