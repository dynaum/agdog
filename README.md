# agdog

`htop` and `nvitop`, fused and agent-aware. One terminal pane that groups CPU, RAM, GPU, and VRAM by the agent that owns it, and tells you which agent is stuck, runaway, or burning memory.

> Status: early. Design spec in [`docs/superpowers/specs/2026-07-13-agdog-design.md`](docs/superpowers/specs/2026-07-13-agdog-design.md).

## Why

Run local AI work (ComfyUI renders, a kohya_ss LoRA run, two Ollama models) and every monitor shows PIDs and VRAM but none tells you *which agent* owns the load or whether a job wedged. Memory is exclusive, so one runaway job starves everything, and you find out only when something crashes. agdog answers the real question: **which agent is eating my VRAM right now, and is it stuck?**

## What it does

- Maps CPU, RAM, GPU, and VRAM to agent identity and job type (inference / training / render).
- Classifies each agent: working / stuck / runaway / idle / crashed.
- Flags wedged jobs, runaway loops, and orphan ports.
- Tracks per-agent GPU-time, peak memory, and cost.
- Streams state-change events over a Unix socket so orchestrators can react.

## Backends

- **Apple Silicon** — unified memory + GPU via IOKit / Metal counters.
- **NVIDIA** — per-process VRAM + utilization via NVML.

## Stack

Rust + ratatui. Single static binary.

## License

MIT
