## Why

`aclog sync` 的目标是把本地题解变更和真实在线提交记录关联起来，但当前流程在没有查到提交记录时会直接落成 `chore`，把“用户主动确认的本地修改”误写成了默认行为。现在需要把 `sync` 的交互语义收紧为“先选择，再提交”，避免未确认的自动降级污染训练记录。

## What Changes

- 将 `sync` 的提交流程改为：发现题目文件变更后，总是进入交互式选择界面。
- 移除“无提交记录时自动生成 `chore(...)` commit”的默认行为。
- 保留 `chore` 作为显式用户操作，使其与选择具体提交记录处于同一级决策。
- 为无提交记录场景定义明确的空状态交互，允许用户选择 `chore` 或 `skip`。

## Capabilities

### New Capabilities
- `interactive-sync-selection`: 定义 `sync` 在变更文件、提交记录、空记录场景下的统一交互选择行为

### Modified Capabilities

## Impact

- 影响 `src/cli.rs` 中 `sync` 主流程的决策顺序
- 影响 `src/tui.rs` 中提交记录选择器与空状态展示
- 影响 `src/models.rs` 中 `SyncSelection` 到 commit message 的语义映射
