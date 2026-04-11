## Why

`aclog` 现在已经有 `sync`、`stats` 和 `record browse` 等多个终端工作台，但用户仍然需要先记住子命令，再分别进入这些入口。随着产品定位从“若干命令”转向“训练工作台”，无参启动仍只显示 CLI 帮助已经不再是最顺手的默认路径。

## What Changes

- 将 `aclog` 顶层命令从“必须带子命令”调整为“无参数时直接进入全局训练工作台”。
- 新增一个全局工作台首页，负责展示工作区上下文、当前可恢复的 sync session、最近训练摘要，以及进入 `sync`、`stats`、`record browse`、`record list` 的入口。
- 保留 `aclog --help` / `aclog help` 作为完整 CLI 帮助入口，并继续保留所有现有子命令的直达语义。
- 复用现有统一终端主题、阅读结构和键位语义，让首页成为既有 terminal workbench family 的一员，而不是另一套独立 UI。
- 明确首页只承担导航、摘要和恢复入口，不重新定义 `sync`、`stats`、`browse` 的数据语义，也不引入新的事实源或常驻后台。

## Capabilities

### New Capabilities
- `root-workbench-entry`: 定义 `aclog` 无参启动全局工作台、展示工作区摘要并导航到现有工作流的行为。

### Modified Capabilities
- `terminal-ui-experience`: 统一终端工作台的视觉、布局和导航约定需要扩展到全局首页。

## Impact

- 主要影响 `src/cli.rs` 的顶层解析与无参分发，以及新增的首页 workflow / TUI 入口。
- 需要扩展 `src/app/` 与 `src/ui/terminal/`，让首页能够读取工作区上下文、恢复状态和统计摘要并跳转到现有工作台。
- 需要补充 CLI 解析测试、首页交互测试和入口分发回归测试。
- 不新增外部服务、不引入 WebUI、不改变 `jj` 历史作为训练记录事实源的原则。
