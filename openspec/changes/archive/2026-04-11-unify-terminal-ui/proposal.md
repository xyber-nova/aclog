## Why

当前真实终端界面分散在 `src/tui.rs` 的单一实现中，页面结构、颜色语义和帮助提示都不统一，导致继续演进 `sync`、selector、浏览和统计界面时可维护性与可读性都在下降。现在需要把终端实现统一收口，并把关键信息分区、状态提示和视觉层级做得更接近人类阅读习惯。

## What Changes

- 将真实 ratatui/crossterm 实现统一迁移到 `src/ui/terminal/`，保留 `src/ui/interaction.rs` 作为应用层 UI 抽象，`src/tui.rs` 作为兼容转发层。
- 为所有终端页面引入共享的“训练工作台”设计语言，包括冷静专业的文案语气、中等层级的面板感、以文字标签为主并辅以少量符号的状态表达。
- 为所有终端页面引入共享的主题、布局和帮助提示约定，包括语义配色、选中态、空状态、告警区与快捷键提示。
- 改造 `sync` 预览页和详情页的信息组织，使用户能在进入详情前理解当前项状态，并在详情页同时看到上下文、告警和可执行动作。
- 统一交互式工作流与快捷键语义：`Enter` 用于进入或确认，`Esc` 用于返回上一层，`q` 用于退出当前工作台，`Tab` 用于同层模式切换，`b` 仅保留为兼容返回别名。
- 让 `sync` 预览页支持安全的快速决策，例如直接标记 `chore`、`skip` 或确认删除，同时保留通过详情页完成 submission 选择的精确路径。
- 改造 `record bind` / `record rebind` 的选择器界面，使其采用统一的上下文区、列表区、详情区和帮助区布局。
- 改造 `record browse` 与 `stats` / review 工作台，使其采用统一的左表右详情结构、显式过滤摘要、页签式模式切换、帮助提示和更清晰的概览层级。
- 在各交互页面增加 `j/k` 导航别名与 `?` 帮助切换，同时保留现有核心键位语义。

## Capabilities

### New Capabilities
- `terminal-ui-experience`: 终端工作台与选择器共享的视觉主题、帮助提示和导航约定。

### Modified Capabilities
- `interactive-sync-selection`: sync 预览与详情界面改为统一信息分区，并显式展示状态、告警、空状态与可执行动作。
- `record-commands`: `record bind` / `record rebind` 的交互选择器采用统一终端布局与导航帮助。
- `record-browser-workbench`: 浏览工作台强化视角标识、过滤摘要、时间线布局和详情区层级。
- `stats-command`: stats 与 review 界面采用统一主题、概览分区、帮助提示和更清晰的钻取入口。

## Impact

- 主要影响 `src/ui/interaction.rs`、新增 `src/ui/terminal/` 模块树、`src/tui.rs` 兼容转发层以及相关 TUI 纯函数测试。
- 不改变 `UserInterface` trait 签名，不改变现有 CLI 命令语义，不引入新的外部运行时依赖。
- onboarding / 新手教程命令不纳入本次 change，待 TUI 主工作流稳定后再单独设计。
- 需要为主题映射、键位兼容、空状态/告警摘要、帮助面板和页面辅助渲染补充测试。
