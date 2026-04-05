## 1. CLI and History Access

- [x] 1.1 Add a `stats` subcommand in `src/cli.rs` and route it to a dedicated `run_stats` flow with `--workspace` support.
- [x] 1.2 Add a read-only `jj` history query in `src/vcs.rs` that collects reachable commit descriptions for local statistics without changing workspace state.

## 2. Stats Parsing and Aggregation

- [x] 2.1 Add stats-oriented models and parsing helpers that recognize project `solve(...)` commit messages and ignore non-matching history entries.
- [x] 2.2 Implement summary aggregation for both total `solve` records and deduplicated per-problem statistics, including verdict and difficulty distributions.
- [x] 2.3 Extend unique-problem aggregation to include algorithm tag distribution.
- [x] 2.4 Upgrade the Luogu tag cache structure to retain tag type metadata and filter non-algorithm tags during stats aggregation.

## 3. Stats TUI

- [x] 3.1 Add a read-only stats screen in `src/tui.rs` that reuses the existing terminal wrapper and renders the overview layout.
- [x] 3.2 Implement the empty-state and exit key handling for the stats screen.
- [x] 3.3 Render algorithm tag distribution in the stats overview screen.

## 4. Verification

- [x] 4.1 Add unit tests for stats message parsing, duplicate-problem aggregation, placeholder handling, and ignored commit types.
- [x] 4.2 Add CLI and TUI-focused tests covering `stats` command dispatch and the no-history empty state.
