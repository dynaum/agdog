# Changelog

All notable changes to agdog are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and versions follow
[SemVer](https://semver.org/).

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
