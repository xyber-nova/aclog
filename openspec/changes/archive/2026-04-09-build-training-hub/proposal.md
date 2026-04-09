## Why

`aclog` 现在已经能把题目文件变更稳定地沉淀成 `solve(...)` 历史，但它仍然偏向“记录器”而不是“训练中枢”：日常同步缺少预览、恢复和防错，记录里也缺少只有用户自己知道的训练上下文，统计结果更难直接支撑复盘和下一步训练决策。现在基础 workflow、测试分层和本地历史解析已经稳定，适合把记录、浏览、复盘和建议这一整条链路补齐。

## What Changes

- 扩展 `sync` 流程，增加批量预览、`--dry-run`、中断恢复、默认候选提示和一致性校验，降低高频记录成本并减少静默记错。
- 扩展 `record` 命令面，为当前记录列表增加过滤与结构化输出，并新增对训练附加信息的查看与编辑入口。
- 为标准 `solve(...)` 记录增加个人训练字段，例如总结、卡点、收获、熟练度、完成方式和本地耗时，并保持对旧记录的向后兼容解析。
- 新增面向复盘的记录浏览 TUI，使用户可以按题目、文件、标签、难度、结果和时间窗口浏览历史，并查看单题时间线。
- 扩展 `stats`，增加时间窗口、首次 AC / 重复练习区分、薄弱标签分析与复习候选题能力，使统计从“展示过去”升级为“支持决策”。
- 保持 CLI-first 边界：所有关键操作仍然必须有非交互 CLI 表达；TUI 负责浏览、筛选、选择和可视化，而不是独占业务能力。

## Capabilities

### New Capabilities
- `training-record-notes`: 为 `solve(...)` 记录增加用户自有训练上下文字段，并支持查看和编辑这些字段。
- `record-browser-workbench`: 提供面向题目历史与文件历史的浏览、筛选和复盘工作台。
- `training-review-suggestions`: 基于本地做题历史和训练字段生成复习候选与训练建议。

### Modified Capabilities
- `interactive-sync-selection`: 扩展同步流程，支持批量预览、dry-run、恢复、默认候选提示和一致性校验。
- `record-commands`: 扩展记录命令，支持列表过滤、结构化输出以及训练记录查看/编辑入口。
- `stats-command`: 扩展统计能力，支持时间窗口、首次 AC / 重复练习区分、钻取和建议入口。

## Impact

- 受影响代码：
  - `src/cli.rs`
  - `src/app/`
  - `src/domain/`
  - `src/ui/` 与 `src/tui.rs`
  - `src/commit_format.rs`
  - `src/vcs/`
  - `src/api/`
- 受影响接口：
  - 扩展 `aclog sync`
  - 扩展 `aclog record list`
  - 新增训练记录维护相关 `record` 子命令
  - 扩展 `aclog stats`
- 数据与兼容性：
  - 标准 `solve(...)` 记录格式将增加新字段
  - 历史解析与统计必须继续兼容旧 commit message
- 测试与验证：
  - 需要为新 CLI 入口、TUI 浏览器、兼容解析、建议逻辑和同步恢复补充离线测试与真实 `jj` 集成测试
