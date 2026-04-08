## 1. Testable Crate Structure and Dependency Boundaries

- [x] 1.1 Add `src/lib.rs` and move reusable module exports there so integration tests can import `app`, `api`, `vcs`, `ui`, and domain helpers without going through the binary entrypoint.
- [x] 1.2 Introduce application-layer dependency interfaces for problem/submission loading, repository access, and non-interactive output, plus default production implementations that delegate to the current `api::*`, `vcs::*`, and stdout behavior.
- [x] 1.3 Refactor `app` workflows to accept injected dependencies and keep existing CLI-facing behavior unchanged.
- [x] 1.4 Split `record list` into a pure render function plus output dispatch so tests can assert rendered text without capturing global stdout.

## 2. Deterministic Workflow Integration Tests

- [x] 2.1 Add `tests/support/` fakes and fixtures for problem providers, repo gateways, UI interactions, output sinks, and common record / metadata builders.
- [x] 2.2 Add workflow tests for `sync` covering unparseable files, active vs deleted changes, and the `submission` / `chore` / `delete` / `skip` decision paths.
- [x] 2.3 Add workflow tests for `record bind` and `record rebind` covering CLI-only paths, mixed CLI+UI fallback, invalid submission / revision errors, and same-problem constraints.
- [x] 2.4 Add workflow tests for `record list` and `stats` covering tracked-file filtering, empty/non-empty rendered output, solve-history parsing, algorithm-tag filtering, and summary delivery to the UI layer.

## 3. Real jj Workspace Integration Coverage

- [x] 3.1 Add temporary-workspace helpers that create real colocated `jj` repositories for integration tests without touching user workspaces.
- [x] 3.2 Add real-`jj` integration tests for repository initialization, changed-problem-file detection, tracked-file checks, commit creation, and commit description rewrite behavior.
- [x] 3.3 Add a real-`jj` integration test that exercises `record list` against actual repository history to confirm fake-based workflow tests stay aligned with repository truth.

## 4. CLI Smoke Tests and CI Gate

- [x] 4.1 Add `dev-dependencies` for `tempfile`, `assert_cmd`, and `predicates`, then add CLI smoke tests for `aclog init`, `aclog stats`, `aclog record list`, and representative error paths.
- [x] 4.2 Add a Linux-only GitHub Actions workflow that runs `cargo fmt --check`, `cargo check`, and `cargo test`, installing `jj` before the test job.
- [x] 4.3 Document the new test entrypoints and default guarantees so contributors know that the default suite is offline-friendly and does not require real Luogu credentials.
