## 1. Global ID And Provider Model

- [x] 1.1 Introduce provider/global-problem-id parsing utilities for Luogu and AtCoder file names, including `unknown:*` normalization for unsupported legacy history.
- [x] 1.2 Extend domain models (`ProblemMetadata`, `SubmissionRecord`, solve/history summaries, browser rows, stats inputs) to carry global IDs, provider, and optional contest context.
- [x] 1.3 Update commit message builders/parsers so new records write global IDs and old Luogu records normalize to `luogu:*` during reads.

## 2. Provider Dispatcher And API Integration

- [x] 2.1 Refactor `src/api/mod.rs` and `ProblemProvider` call paths to accept global IDs and dispatch to provider-specific clients.
- [x] 2.2 Keep Luogu behavior working under the new dispatcher and migrate metadata cache keys/files to global-ID-safe naming.
- [x] 2.3 Add an AtCoder Problems client for metadata, contest context, and user submissions, including graceful fallback for missing difficulty/contest fields.

## 3. Shared Indexing And Record Workflows

- [x] 3.1 Update `RecordIndex` and related aggregations to key problem timelines/current-state views by global ID without cross-provider collisions.
- [x] 3.2 Update `sync` workflow to admit Luogu and AtCoder files, preserve current preview/decision semantics, and show provider information in summary/detail data.
- [x] 3.3 Update `record bind/rebind/show/edit/list` to resolve provider-aware file targets, keep non-interactive behavior, and render provider/contest context in selectors and outputs.

## 4. Browser And Stats UI

- [x] 4.1 Add provider tabs (`Luogu` / `AtCoder` / `All`) to the browser workbench while keeping files/problems as the second-layer root views via `f` / `p`.
- [x] 4.2 Render provider and contest details throughout browser detail panes and timeline drill-downs, and make provider filters work in text/JSON outputs.
- [x] 4.3 Add provider tabs to stats with `Tab`, keep `o` / `r` / `g` for overview/review/tag-practice mode switching, and ensure AtCoder/All pages show explicit degraded tag sections instead of mixed tag statistics.

## 5. Tests And Change Verification

- [x] 5.1 Expand parser and commit-format tests to cover Luogu global IDs, AtCoder task IDs, and legacy Luogu normalization.
- [x] 5.2 Extend fake deps/support helpers to generate multi-provider metadata, submissions, and mixed-history fixtures.
- [x] 5.3 Add workflow and UI-oriented tests for mixed-provider sync, selector context, browser provider tabs, and stats provider-aware degradation.
- [x] 5.4 Run the relevant OpenSpec and Rust verification commands (`openspec status --change ...`, `cargo test`, and targeted checks) and confirm the change is implementation-ready.
