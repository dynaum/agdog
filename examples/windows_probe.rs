//! Temporary diagnostic probe. Runs on a real Windows host (GitHub Actions
//! `windows-latest`) and reports what the platform actually supports, so the
//! remaining Windows work is scoped from evidence rather than assumption.
//!
//! Delete once the findings are folded into real tests.

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("windows-only probe; nothing to do on this host");
}

#[cfg(target_os = "windows")]
fn main() {
    use agdog::collect::gpu::{GpuCollector, default_gpu_collector};
    use agdog::collect::system::SystemCollector;
    use std::path::PathBuf;

    // Child mode: a spawned copy of this binary, renamed to look like an agent.
    if std::env::args().any(|a| a == "--sleep") {
        std::thread::sleep(std::time::Duration::from_secs(25));
        return;
    }

    println!("=== ENVIRONMENT ===");
    println!("USERPROFILE   = {:?}", std::env::var("USERPROFILE").ok());
    println!("HOME          = {:?}", std::env::var("HOME").ok());
    println!("temp_dir      = {:?}", std::env::temp_dir());
    println!(
        "XDG_RUNTIME_DIR = {:?}",
        std::env::var("XDG_RUNTIME_DIR").ok()
    );
    println!("cwd           = {:?}", std::env::current_dir().ok());

    println!("\n=== GPU BACKEND SELECTION ===");
    let mut gpu = default_gpu_collector();
    println!("selected backend = {}", gpu.name());
    let samples = gpu.sample();
    println!("device count     = {}", samples.len());
    for s in &samples {
        println!(
            "  gpu{} util={}% mem={}/{} MiB temp={}C power={}W per_pid={}",
            s.index,
            s.util_pct,
            s.mem_used / 1024 / 1024,
            s.mem_total / 1024 / 1024,
            s.temp_c,
            s.power_w,
            s.per_pid.len()
        );
    }

    println!("\n=== DXGI BACKEND (direct) ===");
    match agdog::collect::gpu_windows::WindowsGpu::try_new() {
        Some(mut w) => {
            let s = w.sample();
            println!("DXGI available, adapters = {}", s.len());
            for d in &s {
                println!(
                    "  adapter{} util={}% vram={}/{} MiB",
                    d.index,
                    d.util_pct,
                    d.mem_used / 1024 / 1024,
                    d.mem_total / 1024 / 1024
                );
            }
        }
        None => println!("DXGI unavailable (no hardware adapter enumerated)"),
    }

    println!("\n=== NVML BACKEND (direct) ===");
    match agdog::collect::gpu_nvml::NvmlGpu::try_new() {
        Some(_) => println!("NVML available (driver present)"),
        None => println!("NVML unavailable (expected on a GPU-less runner)"),
    }

    // Rename a copy of this binary to `claude.exe` and run it, to prove whether
    // attribution resolves a real Windows process to an agent.
    println!("\n=== ATTRIBUTION (live process) ===");
    let me = std::env::current_exe().expect("current_exe");
    let fake: PathBuf = std::env::temp_dir().join("claude.exe");
    std::fs::copy(&me, &fake).expect("copy to claude.exe");
    let mut child = std::process::Command::new(&fake)
        .arg("--sleep")
        .spawn()
        .expect("spawn claude.exe");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let mut sys = SystemCollector::new();
    let _ = sys.sample(); // first pass primes CPU deltas
    std::thread::sleep(std::time::Duration::from_millis(500));
    let samples = sys.sample();
    let target = samples.iter().find(|s| s.pid == child.id());
    match target {
        Some(s) => {
            println!("spawned pid   = {}", s.pid);
            println!("exe_name      = {:?}", s.exe_name);
            println!("cmd           = {:?}", s.cmd);
            println!("cwd           = {:?}", s.cwd);
            println!(
                "cli_signature = {:?}",
                agdog::attribute::cli_signature(&s.exe_name.to_lowercase())
            );
            println!(
                "agent_root    = {:?}",
                agdog::attribute::agent_root(s, None)
            );
        }
        None => println!("spawned process not found in samples (pid {})", child.id()),
    }
    let _ = child.kill();

    println!("\n=== TRANSCRIPT DIR (Tier 2) ===");
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let projects = PathBuf::from(&home).join(".claude").join("projects");
    println!("projects dir  = {:?} exists={}", projects, projects.is_dir());
    if let Ok(rd) = std::fs::read_dir(&projects) {
        for e in rd.flatten().take(20) {
            println!("  entry: {:?}", e.file_name());
        }
    }
}
