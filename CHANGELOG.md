# Changelog

All notable changes to agdog are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and versions follow
[SemVer](https://semver.org/).

## [0.3.1] - 2026-07-15

### Fixed
- Empty agents table on a machine with no agent CLIs running. `unassigned`
  (the process list) now shows automatically when no agents are detected, and
  a truly empty view shows a hint instead of a blank box. Fixes "no data" on a
  fresh install.

## [0.3.0] - 2026-07-15

### Added
- **Host CPU panel**, giving CPU the same billing as the GPUs: an overall
  load-colored utilization bar, a 60-second CPU sparkline, a per-core heatmap
  (one cell per logical core, colored by load), and the 1 / 5 / 15-minute load
  average in the title. Backed by new per-core and load-average collection.
- **Demo mode** (`AGDOG_DEMO=1`): curated mock agents and GPUs for screenshots
  and a first run on a host with no agents. Drives the README GIF.

### Changed
- **Subagent rows** now carry their own CPU%, memory, and task instead of only
  name / state / source. Process subagents show real metrics; transcript- and
  socket-sourced ones leave unknown cells blank rather than invent a number.

## [0.2.0] - 2026-07-14

### Added
- **Windows support.** Real GPU data via NVML (NVIDIA) and DXGI + PDH (VRAM and
  utilization for any adapter). A Windows binary is built in the release
  pipeline and installed with a one-liner:
  `irm https://raw.githubusercontent.com/dynaum/agdog/master/install.ps1 | iex`.

### Changed
- Backends are selected by OS with no feature flags on all three platforms.
- The event socket is Unix-only for now; on Windows it is stubbed (a loopback
  transport is planned).

## [0.1.5] - 2026-07-14

### Added
- **Subagents**, nested under their parent agent, from three sources: child
  agent processes (via the process tree), Claude Code Task sidechains read from
  session transcripts (`isSidechain`), and agents that report over the socket.
  New `SUB` column and indented `↳` rows.
- **Per-agent CPU bar**, scaled to the busiest visible agent, so you can see who
  is using the most at a glance.

## [0.1.4] - 2026-07-14

### Added
- Visible sort indicator: a cyan `▾` marks the active sort column.
- `COST` column, so every sort key maps to a visible column.

## [0.1.3] - 2026-07-14

### Added
- Agents-first view: the `unassigned` catch-all is hidden by default; press `a`
  to show or hide it.
- vi-style `j` / `k` navigation (alongside the arrow keys).

### Changed
- Summary counts exclude `unassigned`.

## [0.1.2] - 2026-07-14

### Fixed
- Accurate per-session attribution: match agents by program name instead of path
  substrings, exclude `.app` GUI bundles and `/System` services, name sessions by
  project directory, and roll child processes into their session via the tree.

### Added
- `agdog agents` command to print detected agents once and exit.

## [0.1.1] - 2026-07-14

### Changed
- The GPU backend is auto-selected by OS with no feature flags. macOS reads real
  GPU utilization from IOKit (`ioreg`) without sudo, plus unified memory.

## [0.1.0] - 2026-07-13

### Added
- Initial release. Agent-aware terminal resource monitor: real process, CPU, and
  memory metrics; mock / NVIDIA (NVML) / macOS GPU backends; process-to-agent
  attribution; state classification; grouped TUI matching the design mockup;
  Unix-socket event API and `agdog watch`; cost, sort, and filter; MIT license,
  GitHub Actions CI, and a Homebrew tap.
