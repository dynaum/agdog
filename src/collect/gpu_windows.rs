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
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::core::{Interface, PCWSTR};

/// A PDH query kept open across ticks.
///
/// `\GPU Engine(*)\Utilization Percentage` is a rate counter: a value only
/// exists relative to a previous collection. Opening a fresh query per sample
/// meant paying a blocking 200ms sleep every tick just to manufacture that
/// interval, on the same thread that draws the UI. Holding the query open lets
/// the refresh interval itself provide the spacing, so sampling is immediate
/// and the rate is measured over the real elapsed second.
struct PdhQuery {
    query: isize,
    counter: isize,
}

impl Drop for PdhQuery {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.query);
        }
    }
}

/// DXGI + PDH GPU collector for Windows (non-NVIDIA or NVML-unavailable hosts).
pub struct WindowsGpu {
    /// None when the GPU engine counters are unavailable on this host, in
    /// which case utilization reports 0 and VRAM still works.
    util: Option<PdhQuery>,
}

impl WindowsGpu {
    /// Available whenever at least one hardware adapter is enumerable.
    pub fn try_new() -> Option<Self> {
        if dxgi_adapters().is_empty() {
            return None;
        }
        Some(WindowsGpu {
            util: open_util_query(),
        })
    }
}

impl GpuCollector for WindowsGpu {
    fn sample(&mut self) -> Vec<GpuSample> {
        // One overall utilization figure; applied to each adapter (fine for the
        // common single-GPU case). Reads 0 on the very first tick, before a
        // baseline collection exists to compute a rate against.
        let util = self
            .util
            .as_ref()
            .and_then(|q| unsafe { read_util(q) })
            .unwrap_or(0.0);
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

            let desc: DXGI_ADAPTER_DESC1 = match adapter1.GetDesc1() {
                Ok(d) => d,
                Err(_) => continue,
            };

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

/// Open the GPU engine utilization query once and take a baseline reading.
///
/// Returns None when the counter set is missing, which is normal on a host
/// with no GPU driver exposing it.
fn open_util_query() -> Option<PdhQuery> {
    unsafe {
        let mut query: isize = 0;
        if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
            return None;
        }
        let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage\0"
            .encode_utf16()
            .collect();
        let mut counter: isize = 0;
        if PdhAddEnglishCounterW(query, PCWSTR(path.as_ptr()), 0, &mut counter) != 0 {
            let _ = PdhCloseQuery(query);
            return None;
        }
        // Baseline collection; the next one produces the first real rate.
        if PdhCollectQueryData(query) != 0 {
            let _ = PdhCloseQuery(query);
            return None;
        }
        Some(PdhQuery { query, counter })
    }
}

/// Read utilization since the previous collection. Sums the per-engine
/// instances (3D, Copy, Video, ...) and clamps to a single 0..100 figure.
unsafe fn read_util(q: &PdhQuery) -> Option<f32> {
    unsafe {
        let (query, counter) = (q.query, q.counter);
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
