## 1. Change Classification

- [x] 1.1 Extend sync change collection to return each problem-file path together with whether it is deleted or still active
- [x] 1.2 Update the sync main flow to branch on deletion before submission lookup, while preserving problem-id extraction and metadata loading

## 2. Delete Confirmation Flow

- [x] 2.1 Add a dedicated delete-confirmation selection result and wire it through commit planning
- [x] 2.2 Implement a delete-specific TUI that shows detected deletion context and supports confirm-delete or skip only
- [x] 2.3 Add deletion-specific commit message generation that keeps problem context but clearly represents file maintenance

## 3. Verification

- [x] 3.1 Add tests covering deleted vs non-deleted change classification and sync planning behavior
- [x] 3.2 Add tests for delete confirmation input handling and deletion commit message generation
- [x] 3.3 Run the relevant test suite and confirm deleted files no longer enter submission selection
