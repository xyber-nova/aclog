## Why

`aclog sync` 目前把删除文件和普通题解变更放在同一条选择路径里处理，导致删除题目文件时仍会去查询提交记录并要求用户在“绑定提交 / chore / skip”的语义中做选择。这会把“维护本地文件”的动作误表达为“处理做题记录”，使交互语义和生成的 commit message 都失真。

## What Changes

- 为 `sync` 增加删除文件的专用处理分支，将删除动作从提交记录绑定流程中拆分出来。
- 检测到删除题目文件时，继续保留题目上下文，但不再查询提交记录，也不再展示提交记录列表。
- 为删除文件提供专用确认界面，允许用户显式确认删除或跳过当前文件。
- 为删除动作生成独立的维护类 commit message，明确表示这是题目文件删除，而不是做题记录或 `chore`。

## Capabilities

### New Capabilities
- `sync-delete-flow`: 定义 `sync` 在检测到题目文件删除时的确认交互、上下文保留和 commit 语义

### Modified Capabilities

## Impact

- 影响 `src/vcs.rs` 中 `sync` 变更收集结果，需要区分删除与非删除文件
- 影响 `src/cli.rs` 中 `sync` 主流程的上下文获取和分支决策
- 影响 `src/tui.rs` 与 `src/models.rs` 中删除确认交互和维护类 commit message 生成
