# agdog TUI mockup reference

The approved high-fidelity mockup was built in Claude Design:
https://claude.ai/design/p/8b8b293e-d06d-42e9-97b3-b075db0dc8e5

Layout the implementation (Task 10) must match, top to bottom:

1. **Title bar** — `agdog vX.Y.Z` · date · driver · CUDA/version · node · uptime. Right side: window title `agdog — <host> — <cols>x<rows>`.
2. **Aggregate strip** — `agents N working · N idle · N stuck · N runaway/crashed | gpu NN% vram U/T GB | cpu NN% ram U/T GB | cost $X/hr · $Y today`.
3. **Per-GPU panels** — one box per GPU: model + mem + `P0 <temp>C <power>W / <cap>W <procs> proc`, then `UTL` and `MEM` horizontal bars with percent/value.
4. **Host panel** — `HOST · <cpu model>`: per-core meters (two columns) plus `Mem`, `Swap`, `Tasks`, `Load`, `Disk r/w`, `Net`.
5. **Agents table** — columns: `PID · AGENT · KIND · GPU · VRAM GB · GPU% · CPU% · MEM · COST · UPTIME · STATE · TASK`. One row per resource-owning process, grouped by agent. Selected row highlighted.
6. **Detail pane** (bottom-left) — selected agent: `gpu util · last 60s` sparkline, then `step/epoch/loss/vram/gpu-time/cost/eta` and a short log tail.
7. **Alerts pane** (bottom-right) — one line per `RUNAWAY`/`CRASHED`/`STUCK`/idle-reap alert with the F-key action.
8. **Footer** — keybinds (`q · ↑↓ · s sort · / filter`) and `agdog.sock: N subscribers`.

State colors: Working=green, Idle=gray, Stuck=yellow, Runaway=red, Crashed=magenta.
