<div align="center">

# 🐕 agdog

**`htop` and `nvitop`, fused and agent-aware.**

One terminal pane that groups CPU, RAM, GPU, and VRAM by the *agent* that owns each process, then tells you which one is stuck, runaway, or eating your memory.

[![CI](https://github.com/dynaum/agdog/actions/workflows/ci.yml/badge.svg)](https://github.com/dynaum/agdog/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-edition%202024-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)

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

## Install

**Homebrew** (macOS arm64/x86_64, Linux x86_64):

```bash
brew install dynaum/tap/agdog
```

**Windows** (one-liner, no prerequisites):

```powershell
irm https://raw.githubusercontent.com/dynaum/agdog/master/install.ps1 | iex
```

Downloads the latest release, installs `agdog.exe` to `%LOCALAPPDATA%\agdog`, and adds it to your PATH.

**From source** (requires Rust 1.85+ for edition 2024):

```bash
cargo install --git https://github.com/dynaum/agdog
```

Prebuilt binaries are attached to every [release](https://github.com/dynaum/agdog/releases):

| Platform | Targets |
|----------|---------|
| macOS    | `aarch64-apple-darwin`, `x86_64-apple-darwin` |
| Linux    | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (glibc 2.34+, so Debian 12, Ubuntu 22.04, RHEL/Rocky 9 and Amazon Linux 2023 all work), `x86_64-unknown-linux-musl` (fully static, Alpine) |
| Windows  | `x86_64-pc-windows-msvc` |

Every archive is listed in the `SHA256SUMS` file published with the release.

## Quick start

```bash
agdog
```

That's it. agdog detects your machine and shows **real numbers automatically**, no flags, no sudo:

- **Mac** — live GPU utilization and unified memory via IOKit.
- **Linux / Windows with an NVIDIA GPU** — per-process VRAM and utilization via NVML.
- **Windows (any GPU)** — VRAM via DXGI and utilization via PDH counters.
- **Anywhere else** — a mock GPU keeps the view alive so the tool still runs.

## Usage

```bash
agdog                      # live monitor
agdog --interval 2         # refresh every 2 seconds
agdog --gpu-hourly 2.50    # derive per-agent cost at $2.50 / GPU-hour
agdog watch                # subscribe to the event socket, print JSON events
agdog agents               # print the detected agents once and exit
```

agdog identifies agents by their **program name**, not path substrings, so it separates real CLIs (each `claude` session, `ollama`, `aider`, ...) from GUI apps and system services. Parallel sessions are named by their project directory (`claude:myproject`); child processes (node, MCP servers) roll into their session; everything else lands in one `unassigned` row.

Keys inside the TUI:

| Key       | Action                          |
|-----------|---------------------------------|
| `q`       | quit                            |
| `j` / `k` | select an agent (also `↑` / `↓`) |
| `s`       | cycle sort column (gpu/cpu/mem/cost/name) |
| `a`       | show/hide other processes (the `unassigned` row) |
| `/`       | filter by agent name            |

By default the table shows only your agents; press `a` to also show the `unassigned` row (everything that isn't an agent: the OS, GUI apps, background daemons).

## How it works

**Attribution** is the core idea. agdog maps each process to an agent with layered heuristics, highest confidence first:

1. An explicit `AGENT_ID` environment tag. Export it and the process is grouped
   under that name whatever the binary is called:
   `AGENT_ID=render-batch ./my-worker`. **Linux only.** macOS refuses to expose
   another process's environment, even a child of the reading process, so the
   tag is invisible there and attribution falls through to the signature below.
2. Command-line signatures (`comfyui`, `kohya`, `ollama`, `vllm`, `claude`, ...).
3. Process-tree ancestry, so child workers inherit their parent's agent.

**Classification** watches utilization and hold-time to label each agent `working`, `idle`, `stuck` (memory held with no activity), `runaway` (pegged and sustained), or `crashed`.

**Subagents** nest under their parent agent (a `SUB` count on the parent, indented `↳` rows). agdog finds them three ways: child agent *processes* via the tree, Claude Code Task sidechains read from the session transcript (`isSidechain`), and agents that report their subagents over the socket.

**The socket API** is what makes agdog agent-native rather than a dashboard. agdog exposes a Unix socket that streams state-change events as JSON lines. An orchestrator, or the agents themselves, subscribe and react to a stuck or runaway job instead of scraping the screen. Unix only for now: on Windows the socket is stubbed and `agdog watch` exits with an unsupported error.

```bash
$ agdog watch
{"kind":"state_changed","agent_id":"kohya-lora","from":"working","to":"stuck","ts_secs":842}
{"kind":"state_changed","agent_id":"vllm-serve","from":"working","to":"runaway","ts_secs":905}
```

## Backends

| Backend       | Selected on         | What you get |
|---------------|---------------------|--------------|
| Apple Silicon | macOS (auto)        | GPU utilization via IOKit `ioreg` + unified memory via sysinfo. No sudo. |
| NVIDIA        | Linux / Windows (auto) | Per-process VRAM and utilization via NVML (runtime driver load). |
| DXGI + PDH    | Windows (auto)      | Per-adapter VRAM via DXGI and utilization via PDH counters (any GPU). |
| None          | fallback            | No GPU panel. Shown when no real backend initializes. |
| Mock          | `AGDOG_DEMO=1` only | Deterministic fake GPUs, for screenshots and the demo GIF. |

The backend is chosen by your OS at build time. When no real backend initializes (no discrete GPU, or a driver that failed to load) agdog reports **no readable GPU** rather than inventing devices. Fabricated GPU numbers appear only under `AGDOG_DEMO=1`, and the footer labels them `mock (simulated)`. The live backend name is always shown in the footer.

## Status

Early but complete and runnable. Real process metrics work today on all three platforms.

Verified per platform:

- **macOS** — fully exercised. Attribution, per-core CPU, and GPU utilization via `ioreg` all read real data. Covered by CI.
- **Linux** — process, CPU, and memory verified in CI. The NVML GPU path has no hardware coverage yet, so it is untested against a real card.
- **Windows** — builds and passes tests in CI, but has no hardware coverage. The DXGI/PDH GPU backend is untested. The event socket is Unix-only, so `agdog watch` and socket-reported subagents are unavailable. Transcript-based subagent detection (Tier 2) does not yet resolve Windows paths.

Contributions and hardware reports welcome, particularly from NVIDIA Linux and Windows hosts.

## Stack

Rust · [ratatui](https://ratatui.rs) · [sysinfo](https://crates.io/crates/sysinfo) · [nvml-wrapper](https://crates.io/crates/nvml-wrapper). A single binary with no install-time dependencies. The musl build is fully static. The glibc and macOS builds link the system libc, the NVIDIA backend loads the driver library at runtime, and the macOS backend shells out to `ioreg`.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

[MIT](LICENSE) © Elber Ribeiro
