## 1. Record Model and Parsing

- [x] 1.1 Extend the `solve(...)` record model and `commit_format` parser/builder to support optional training fields (`Note`, `Mistakes`, `Insight`, `Confidence`, `Source-Kind`, `Time-Spent`) while preserving backward compatibility with existing history.
- [x] 1.2 Add shared constants and normalization helpers for new training fields so `bind`, `rebind`, `edit`, browser, and stats use one canonical field vocabulary.
- [x] 1.3 Introduce a shared record index/read model that can derive per-file current state, per-problem current state, and file/problem timelines from local `jj` history.

## 2. Sync Workflow Upgrade

- [x] 2.1 Extend the `sync` workflow and CLI surface with `--dry-run` and recovery control flags, keeping all write operations behind explicit non-dry-run execution.
- [x] 2.2 Add batch preview state for `sync`, including pending item summaries, default candidate hints, empty-submission states, and per-item status tracking.
- [x] 2.3 Persist unfinished sync batches under `.aclog` and implement resume/rebuild behavior with validity checks against current workspace state.
- [x] 2.4 Add pre-commit consistency warnings for mismatched problem IDs, duplicate submission bindings, and other recoverable sync risks.
- [x] 2.5 Expand sync TUI/UI abstractions to support preview navigation and warning display while preserving explicit `submission` / `chore` / `skip` / `delete` decisions.

## 3. Record Commands and Training Notes

- [x] 3.1 Extend `record list` with file/problem/verdict/difficulty/tag filters and structured output, all backed by the shared current-record index.
- [x] 3.2 Add `record show <file>` to display the latest or explicitly targeted historical `solve(...)` record with full metadata and training fields.
- [x] 3.3 Add `record edit <file>` to rewrite training fields for the latest or explicitly targeted record without modifying solution file contents.
- [x] 3.4 Update `record rebind` so submission rewrites preserve any existing training fields on the selected record.

## 4. Browser Workbench

- [x] 4.1 Introduce browser-oriented view models for file view, problem view, and timeline view based on the shared record index.
- [x] 4.2 Implement the record browser TUI with filtering, list/detail navigation, and timeline drilldown for files and problems.
- [x] 4.3 Wire stats and suggestion entries to the browser workbench so drilldowns reuse the same underlying history model and detail rendering.

## 5. Stats and Review Suggestions

- [x] 5.1 Extend stats aggregation to support time-window filtering, first-AC vs repeated-practice distinctions, and window-aware summaries without losing full-history state semantics.
- [x] 5.2 Implement deterministic review suggestion generation for `stale`, `retry`, and `weakness` candidates using local history and training-note signals.
- [x] 5.3 Expand the stats UI/CLI flow to expose review mode, suggestion explanations, and drilldown entry points into related records or topics.

## 6. Verification and Documentation

- [x] 6.1 Add unit tests for new record parsing/building rules, backward compatibility, and shared index semantics such as latest-by-file, latest-by-problem, and timeline ordering.
- [x] 6.2 Add workflow tests for sync dry-run, sync resume, duplicate/mismatch warnings, `record show`, `record edit`, filtered/structured `record list`, and rebind preservation of training fields.
- [x] 6.3 Add TUI-facing tests or presenter tests for browser, stats drilldown, and suggestion explanations using fake UI dependencies where possible.
- [x] 6.4 Add or update real `jj` integration tests for rewrite-with-notes, sync resume state, and history reads used by browser/stats flows.
- [x] 6.5 Update user-facing docs and repository guidance to document the new record fields, sync preview/recovery behavior, browser workflow, and stats/review capabilities.
