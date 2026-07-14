<div align="center">

# 🐕 agdog

**`htop` and `nvitop`, fused and agent-aware.**

One terminal pane that groups CPU, RAM, GPU, and VRAM by the *agent* that owns each process, then tells you which one is stuck, runaway, or eating your memory.

[![CI](https://github.com/dynaum/agdog/actions/workflows/ci.yml/badge.svg)](https://github.com/dynaum/agdog/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-edition%202024-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey.svg)

![agdog demo](docs/demo.gif)

</div>

---

## The problem

You run local AI work: a ComfyUI render, a kohya_ss LoRA training run, two Ollama models, a couple of coding agents. Every monitor on your machine shows you PIDs, CPU%, and a wall of VRAM numbers. None of them tells you *which agent* owns that load, whether a job wedged, or what it cost.

Memory is exclusive. One runaway render starves everything else, and you find out only when something crashes at 2am. `htop` shows a process. `nvitop` shows a device. Neither shows you the **agent**.

agdog answers the question you actually have:

> **Which agent is eating my VRAM right now, and is it stuck?**

## What you see

agdog reads your real processes, groups them by the agent that spawned them, and paints one live pane:

- **Aggregate strip** — working / idle / stuck / runaway counts, total GPU / VRAM / CPU / RAM, and cost per hour.
- **Per-GPU panels** — nvitop-style UTL and MEM bars with temperature and power.
- **Agents table** — one row per agent, grouped from its processes: kind (render / train / infer / coding), GPU%, VRAM, CPU%, memory, cost, uptime, and a color-coded state.
- **Detail pane** — a 60-second GPU sparkline plus stats for the selected agent.
- **Alerts** — every runaway, crashed, or stuck agent with a one-key action.

State colors tell the story at a glance: 🟢 working · ⚪ idle · 🟡 stuck · 🔴 runaway · 🟣 crashed.

## Quick start

```bash
# Requires Rust 1.85+ (edition 2024)
git clone https://github.com/dynaum/agdog && cd agdog
cargo install --path .

agdog
```

That runs today on any machine: real process, CPU, and memory metrics, with a mock GPU backend so the view is always alive. Add a real GPU backend when you want live device data:

```bash
cargo install --path . --features nvml     # NVIDIA (Linux)
cargo install --path . --features macos    # Apple Silicon
```

## Usage

```bash
agdog                      # live monitor
agdog --interval 2         # refresh every 2 seconds
agdog --gpu-hourly 2.50    # derive per-agent cost at $2.50 / GPU-hour
agdog watch                # subscribe to the event socket, print JSON events
```

Keys inside the TUI:

| Key   | Action                          |
|-------|---------------------------------|
| `q`   | quit                            |
| `↑ ↓` | select an agent                 |
| `s`   | cycle sort column (gpu/cpu/mem/cost/name) |
| `/`   | filter by agent name            |

## How it works

**Attribution** is the core idea. agdog maps each process to an agent with layered heuristics, highest confidence first:

1. An explicit `AGENT_ID` environment tag.
2. Command-line signatures (`comfyui`, `kohya`, `ollama`, `vllm`, `claude`, ...).
3. Process-tree ancestry, so child workers inherit their parent's agent.

**Classification** watches utilization and hold-time to label each agent `working`, `idle`, `stuck` (memory held with no activity), `runaway` (pegged and sustained), or `crashed`.

**The socket API** is what makes agdog agent-native, not just a dashboard. agdog exposes a Unix socket that streams state-change events as JSON lines. An orchestrator, or the agents themselves, can subscribe and react to a stuck or runaway job instead of scraping the screen:

```bash
$ agdog watch
{"kind":"state_changed","agent_id":"kohya-lora","from":"working","to":"stuck","ts_secs":842}
{"kind":"state_changed","agent_id":"vllm-serve","from":"working","to":"runaway","ts_secs":905}
```

## Backends

| Backend      | Feature flag        | What you get |
|--------------|---------------------|--------------|
| Mock         | *(default)*         | Deterministic fake GPUs so the binary runs anywhere. |
| NVIDIA       | `--features nvml`   | Per-process VRAM and utilization via NVML (runtime driver load). |
| Apple Silicon| `--features macos`  | Unified-memory totals via sysinfo, GPU utilization via `powermetrics` (needs sudo). |

Every backend falls back to the mock when its hardware is absent, so agdog always runs.

## Status

Early but complete and runnable. Real process metrics work today; the mock GPU backend keeps the UI alive on any machine; the NVIDIA and macOS backends are wired behind their feature flags. Contributions and hardware reports welcome.

## Stack

Rust · [ratatui](https://ratatui.rs) · [sysinfo](https://crates.io/crates/sysinfo) · [nvml-wrapper](https://crates.io/crates/nvml-wrapper). Single static binary, no runtime dependencies.

## License

[MIT](LICENSE) © Elber Ribeiro
