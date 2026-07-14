# agdog

[![CI](https://github.com/dynaum/agdog/actions/workflows/ci.yml/badge.svg)](https://github.com/dynaum/agdog/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`htop` and `nvitop`, fused and agent-aware. One terminal pane that groups CPU, RAM, GPU, and VRAM by the agent that owns it, and tells you which agent is stuck, runaway, or burning memory.

> Status: early, but runnable. Design spec in [`docs/superpowers/specs/2026-07-13-agdog-design.md`](docs/superpowers/specs/2026-07-13-agdog-design.md).

<!-- TODO: replace with an asciinema/GIF of agdog running. -->

## Install

```bash
# From source (requires Rust 1.85+ for edition 2024):
git clone https://github.com/dynaum/agdog && cd agdog
cargo install --path .
```

## Why

Run local AI work (ComfyUI renders, a kohya_ss LoRA run, two Ollama models) and every monitor shows PIDs and VRAM but none tells you *which agent* owns the load or whether a job wedged. Memory is exclusive, so one runaway job starves everything, and you find out only when something crashes. agdog answers the real question: **which agent is eating my VRAM right now, and is it stuck?**

## What it does

- Maps CPU, RAM, GPU, and VRAM to agent identity and job type (inference / training / render).
- Classifies each agent: working / stuck / runaway / idle / crashed.
- Flags wedged jobs, runaway loops, and orphan ports.
- Tracks per-agent GPU-time, peak memory, and cost.
- Streams state-change events over a Unix socket so orchestrators can react.

## Usage

```bash
agdog                      # live monitor (mock GPU by default)
agdog --interval 2         # refresh every 2 seconds
agdog --gpu-hourly 2.50    # derive per-agent cost at $2.50/GPU-hour
agdog watch                # subscribe to the event socket, print JSON events
```

In the TUI: `q` quit · `↑↓` select · `s` cycle sort column · `/` filter by agent name.

Build with a real GPU backend:

```bash
cargo build --release --features nvml    # NVIDIA (Linux)
cargo build --release --features macos   # Apple Silicon (GPU util needs sudo)
```

## Backends

- **Mock** (default) — deterministic fake GPUs so the binary runs anywhere.
- **NVIDIA** (`--features nvml`) — per-process VRAM + utilization via NVML.
- **Apple Silicon** (`--features macos`) — unified memory via sysinfo, GPU util via powermetrics.

## Stack

Rust + ratatui. Single static binary.

## License

MIT
