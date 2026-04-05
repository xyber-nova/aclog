## 1. CLI Surface and Validation

- [x] 1.1 Add a `record` subcommand group in `src/cli.rs` with `bind <file>`, `rebind <file>`, and `list`, plus non-interactive flags for CLI-only selection (`--submission-id`, `--record-rev`).
- [x] 1.2 Validate that `bind` and `rebind` target an existing, parseable solution file, and reject files that are not tracked by the current `jj` workspace.

## 2. Record History and Models

- [x] 2.1 Add file-oriented history queries in `src/vcs.rs` / `src/models.rs` that find standard `solve(...)` records by normalized file path and surface the latest record per file for `record list`.
- [x] 2.2 Add helpers that rebuild standard `solve(...)` commit messages from a chosen file, problem metadata, and submission, so `bind` and `rebind` share one message-construction path.

## 3. Interactive and Non-Interactive Flows

- [x] 3.1 Implement `record bind` orchestration that fetches metadata and submissions, uses non-interactive `--submission-id` when present, and otherwise falls back to the existing submission selector TUI.
- [x] 3.2 Implement `record rebind` orchestration that locates candidate history entries for the target file, uses `--record-rev` / `--submission-id` when present, and otherwise drives the remaining unresolved choices through TUI.
- [x] 3.3 Add a pure record-selection TUI in `src/tui.rs` for choosing which historical `solve(...)` entry to rewrite, while keeping `record list` as plain CLI output.
- [x] 3.4 Add a `jj` rewrite helper that rewrites only the selected standard `solve(...)` commit and preserves same-file / same-problem constraints.

## 4. Verification and Documentation

- [x] 4.1 Add tests covering bind/rebind/list command parsing, tracked-file validation, same-file latest-record indexing, and same-problem rebind constraints.
- [x] 4.2 Add tests covering CLI-only bind/rebind paths, mixed CLI+TUI fallback behavior, and the no-history error path for `record rebind`.
- [x] 4.3 Update repository docs, including `AGENTS.md`, to document that `aclog` is a CLI tool with an optional TUI interaction mode and that all selection steps must be expressible through non-interactive CLI inputs.
