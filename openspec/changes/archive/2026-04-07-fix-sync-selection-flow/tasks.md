## 1. Sync Flow

- [x] 1.1 Remove the implicit `no submissions => chore` branch from the sync selection path
- [x] 1.2 Ensure `run_sync` only creates commit plans from explicit `Submission` or `Chore` selections
- [x] 1.3 Tighten commit message generation so `chore(...)` is produced only from an explicit chore selection

## 2. TUI Behavior

- [x] 2.1 Update the submission selector to open even when the submission list is empty
- [x] 2.2 Add an empty-records state that explains no submissions were found and shows available keys
- [x] 2.3 Keep `Enter` for submission selection, `c` for chore, and `Esc` for skip with behavior consistent across files

## 3. Verification

- [x] 3.1 Add tests for files with submission records selecting submission, chore, and skip
- [x] 3.2 Add tests for files without submission records ensuring no automatic chore commit is produced
- [x] 3.3 Run the relevant test suite and confirm the new sync interaction matches the spec
