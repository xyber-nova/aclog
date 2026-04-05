## Why

`aclog sync` 只能处理当前工作区里已经发生文件变更的题目文件，无法覆盖“漏记一份解法”“这份 cpp 绑错了 submission”“我想先看哪些文件已经记录过”这些日常维护场景。现在已经有稳定的提交选择 TUI 和本地历史解析能力，应该补上独立的 `record` 命令，把解法文件的补录、重绑和浏览做成一等入口。

## What Changes

- 新增 `aclog record` 子命令组，提供 `bind`、`rebind` 和 `list` 三个文件级记录维护能力。
- `bind` 以具体解法文件为操作对象，为该文件手工绑定一条同题 submission，并生成标准 `solve(...)` commit。
- `rebind` 以具体解法文件为操作对象，先选择要重写的既有 `solve(...)` 记录，再选择新的同题 submission，并通过 `jj` rewrite 修正旧记录。
- `list` 按文件列出当前工作区已记录的解法文件状态，而不是按题目聚合。
- 明确 CLI 内部的命令层与交互式终端界面边界：命令层负责参数校验、数据拉取、`jj` 操作与流程编排；交互界面只负责从候选集合里做交互式选择。
- 明确所有通过交互界面完成的选择能力都必须存在等价的非交互 CLI 表达方式；首版固定为 `--submission-id` 和 `--record-rev` 两类选择参数。TUI 只是默认交互模式，而不是 `record` 功能的唯一入口。

## Capabilities

### New Capabilities
- `record-commands`: 为具体解法文件提供手工绑定 submission、重写错误绑定以及按文件查看已记录状态的能力。

### Modified Capabilities

## Impact

- 受影响代码：
  - `src/cli.rs`
  - `src/tui.rs`
  - `src/vcs.rs`
  - `src/models.rs`
  - `src/problem.rs`
- 受影响交互：
  - 新增 `aclog record bind <file>`
  - 新增 `aclog record rebind <file>`
  - 新增 `aclog record list`
- 依赖复用：
  - 继续复用现有洛谷题目元数据与提交记录 API
  - 继续复用现有 ratatui 选择交互与 `jj` 仓库操作
- 文档约束：
  - 需要在 `AGENTS.md` 中明确 CLI 内部命令层与交互模式的边界，并声明所有交互选择都可以用非交互 CLI 参数表达，避免后续实现把 API 或 `jj` 逻辑塞进界面层或把 TUI 变成硬依赖
