# agdog — design spec

Date: 2026-07-13
Status: approved for planning

## One line

`agdog` is to AI agents what `htop` and `nvitop` are to processes and GPUs: a terminal monitor that groups every resource by the agent that owns it and names each agent's real state.

## Problem

Running local AI work (ComfyUI renders, a kohya_ss LoRA training run, two Ollama models, a fleet of coding agents), every monitor shows PIDs, CPU%, and VRAM, but none tells you *which agent* owns the load, whether a job wedged, or what it cost. VRAM and unified memory are exclusive, so one runaway render starves everything, and you find out only when something crashes. You alt-tab between `htop` and `nvitop` and still have to guess.

## What agdog does

- Maps live CPU, RAM, GPU, and VRAM load to agent identity and job type (inference / training / image-render / coding-agent).
- Classifies each agent's semantic state: working / stuck / runaway / idle / crashed.
- Flags wedged jobs (memory held, zero utilization) and runaway loops, plus orphan ports left behind.
- Tracks per-agent GPU-time, peak memory, and derived cost.
- Renders one grouped-by-agent TUI pane, with bars, sparklines, and color-coded state.
- Exposes a Unix-socket subscribe API streaming state-change events as JSON, so orchestrators and agents react instead of only watching.

## Positioning and market

GPU and local-AI first. The CPU/RAM/process columns are free extra dimensions of the same model, not the pitch. This keeps agdog in open water (r/LocalLLaMA, r/comfyui, r/homelab) and out of the red ocean of Claude-Code token trackers (abtop, native Agent View), which already own the coding-agent-process niche.

## Primary user (ICP)

Local AI hobbyists and homelabbers running multiple GPU workloads on one or two machines: ComfyUI power users, kohya_ss LoRA trainers, self-hosted LLM tinkerers. Secondary: small ML teams sharing a workstation.

## Wedge

"Which agent is eating my VRAM right now, and is it stuck?" A single-machine TUI that labels each block of memory by ComfyUI workflow / training run / model and reds out stalled jobs. Nothing else answers this today.

## Architecture

Five units with clean boundaries.

### 1. Collectors (pluggable backends)

- **System collector** — processes, process tree, CPU, RAM. Cross-platform via `sysinfo`.
- **GPU collector** — a trait with two implementations:
  - `nvml` backend for NVIDIA (`nvml-wrapper`): per-process VRAM, SM utilization, GPU-time.
  - `macos` backend for Apple Silicon: unified-memory pressure, GPU utilization, ANE where available, via IOKit / `powermetrics` / Metal performance counters.
- Collectors emit a normalized `ResourceSample` on each tick. Adding a backend never touches the renderer.

### 2. Attribution engine (the moat)

Maps each process to an agent. Layered heuristics, highest confidence first:

1. Explicit tag: `AGENT_ID` / `AGENT_KIND` env var an agent sets (a tiny opt-in SDK/convention).
2. Cmdline signatures: `comfyui`, `kohya`, `accelerate`, `python train.py`, `ollama`, coding-agent binaries.
3. Process-tree ancestry: children inherit the parent's agent.
4. Listening ports: map known local model-server ports to a server identity.

Output: a stable `agent_id`, a `kind`, and a confidence score. Unattributed processes fall into an `unassigned` group.

### 3. State classifier

Per-agent semantic state from utilization deltas and timers:

- `working` — active CPU/GPU utilization.
- `idle` — attributed but near-zero utilization.
- `stuck` — memory held with zero utilization past a threshold.
- `runaway` — sustained max utilization past an expected budget, or a growing child-process count.
- `crashed` — process vanished with a non-clean exit.

### 4. TUI renderer (ratatui)

Grouped-by-agent rows. Per agent: name, kind, state (color-coded), CPU, memory bar, GPU%, VRAM bar, cost, sparkline. A detail pane on selection. Read-only in v1 (no kill actions yet).

### 5. Socket API

Unix socket at `$XDG_RUNTIME_DIR/agdog.sock` (macOS: a temp path). Line-delimited JSON events: `agent_state_changed`, `agent_started`, `agent_exited`, `pressure_warning`. Clients subscribe; agdog never blocks on a slow reader. This is the one herdr-style hook that keeps agdog aligned with the agent-operator pattern.

## Data model (sketch)

```
Agent { id, kind, confidence, state, since }
ResourceSample { pid, agent_id, cpu_pct, rss_bytes, gpu_pct, vram_bytes, ts }
AgentView { agent, cpu_pct, mem_bytes, gpu_pct, vram_bytes, cost, sparkline_ring }
Event { type, agent_id, from_state, to_state, ts }
```

## Stack

Rust + ratatui. Single static binary, `nvml-wrapper` for NVIDIA, IOKit for macOS, aligns with the herdr feel and existing Rust work. `sysinfo` for cross-platform process/CPU/RAM. `crossterm` backend for ratatui. `serde_json` for the socket protocol.

## Milestones

- **M0 — skeleton**: TUI shell with fake data, grouped-by-agent layout matching the approved mockup.
- **M1 — macOS backend**: real system collector + Apple Silicon unified-memory GPU collector. Dogfood on the Mac.
- **M2 — attribution + classifier**: env tag + cmdline + tree heuristics; state machine.
- **M3 — NVIDIA backend**: `nvml` collector behind the same trait.
- **M4 — socket API**: event stream + a `agdog watch` subscriber demo.
- **M5 — polish + launch**: sparklines, cost math, README GIF, Show HN.

## Non-goals (v1)

- No process kill / control actions (observability only).
- No SaaS, no cloud dashboard, no auth.
- No Windows backend.
- No historical database beyond an in-memory ring buffer.

## Distribution

Homebrew tap, single binary, a screenshot of labeled VRAM blocks. r/LocalLLaMA, r/comfyui, r/homelab, Show HN, a ComfyUI custom-node wrapper for reach. Blog post on dynaum.com.

## Risks

- **Attribution must feel magic.** If labeling is wrong or empty, agdog reads as another GPU monitor. The env-tag convention plus strong cmdline signatures mitigate this.
- **macOS GPU access.** No NVML equivalent; `powermetrics` needs sudo, IOKit/Metal counters are the cleaner path. Validate the Apple Silicon backend early (M1).
- **Retention.** Observability-only tools get opened during incidents, not daily. The socket API and runaway-detection push agdog toward always-on.
```
