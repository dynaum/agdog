# Changelog

All notable changes to agdog are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and versions follow
[SemVer](https://semver.org/).

## [0.4.0] - 2026-07-18

Cross-platform honesty and reach. agdog no longer shows numbers it did not
measure, and Windows attribution works.

### Changed
- **A host with no readable GPU now reports exactly that**, instead of falling
  back to the mock backend's four fabricated devices with invented utilization,
  temperature, and wattage. This affected every non-NVIDIA Linux host and any
  NVIDIA host whose driver failed to load: fake hardware was indistinguishable
  from real. Fabricated GPU data now appears only under `AGDOG_DEMO=1`.
- **The footer names the live GPU backend** (`macos`, `nvml`, `windows-dxgi`,
  `none`), and labels demo mode's synthetic data `mock (simulated)`.

### Fixed
- **Windows attribution.** The executable basename is `claude.exe` on Windows,
  and matching it against `claude` failed, so every agent fell into
  `unassigned` and the TUI looked empty. A trailing `.exe` is now stripped.
- **Windows session labels.** Working-directory basenames split on `\` as well
  as `/`, so a session is `claude:proj` rather than `claude:C:\Users\me\proj`.
- **`install.ps1` verifies its download** against the `SHA256SUMS` published
  with the release before extracting. A mismatch, a missing entry, or an
  unreachable checksum file aborts the install.
- **CI on master.** A filter test asserted a condition true only on a host with
  agent CLIs running, so it passed locally and failed on CI. It now seeds its
  own agents rather than depending on the host.

### Added
- **Linux aarch64 and musl release binaries.** The glibc build moved from
  `ubuntu-latest` to `ubuntu-22.04`, dropping the minimum glibc from 2.39 to
  2.35: the previous binary would not run on Debian 12, Ubuntu 22.04, RHEL 9,
  or Amazon Linux 2023. The musl build is fully static, for Alpine.
- **Release smoke tests** run each binary the runner is able to execute, and CI
  cross-compiles the aarch64 and musl targets on every PR so a break surfaces
  before a tag is cut.
- `rust-version = "1.85"`, so an older toolchain reports the MSRV instead of an
  edition 2024 parse error.
- Windows CI now runs `fmt` and `clippy`, matching the Linux and macOS jobs.

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
