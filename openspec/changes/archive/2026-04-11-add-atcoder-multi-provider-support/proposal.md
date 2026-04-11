## Why

`aclog` 当前把 Luogu 作为唯一题目来源，题号解析、远端数据拉取、历史聚合和终端工作台都默认围绕单一 provider 展开。这让用户无法把 AtCoder 题解纳入同一套 `sync`、记录浏览和统计流程，也使未来扩展其他 OJ 时会持续碰到同样的模型边界。

现在引入 AtCoder 支持，可以把系统升级为“多 provider + 全局题目标识”的统一训练记录模型，同时保持对现有 Luogu 历史和交互习惯的兼容。

## What Changes

- 新增多 provider 题目源能力，首版支持 Luogu 与 AtCoder，并为题目、提交、历史记录建立统一的全局题目标识。
- 新生成的 `solve(...)`、`chore(...)`、`remove(...)` 记录统一写入全局题目标识；历史 Luogu 裸题号记录继续兼容解析。
- 新增 AtCoder Problems 非官方 API 接入，用于获取题目元数据、比赛上下文和用户提交记录。
- `sync` 和记录选择器补充 `Source` / `Contest` 展示，但不引入新的 provider 页签。
- 记录浏览工作台和统计工作台引入 provider 页签，并对 AtCoder 采用 provider-aware 的统计降级策略。

## Capabilities

### New Capabilities
- `multi-provider-problem-sources`: 为多 provider 题目元数据、提交记录、全局题目标识和历史兼容解析建立统一行为约定。

### Modified Capabilities
- `interactive-sync-selection`: sync 预览与详情需要展示多 provider 上下文，并允许 AtCoder 题目进入相同工作流。
- `record-commands`: record bind/rebind/show/edit 需要支持多 provider 题目标识与多源选择器上下文。
- `record-browser-workbench`: 浏览工作台需要增加 provider 页签，并在详情中展示来源与比赛信息。
- `stats-command`: 统计工作台需要增加 provider 页签，并对非 Luogu provider 采用降级后的标签与建议口径。
- `jj-history-as-database`: 本地 `jj` 历史中的结构化题目标识需要从单一 Luogu 题号升级为多 provider 全局 ID，同时保留旧记录兼容解析。

## Impact

- 影响 `src/problem.rs`、`src/api/`、`src/app/`、`src/domain/`、`src/commit_format.rs`、`src/ui/terminal/` 等核心路径。
- 新增对 AtCoder Problems 非官方 API 的依赖和相应缓存/限流逻辑。
- 更新多项 OpenSpec 行为规范、测试替身与 workflow 集成测试，覆盖 Luogu + AtCoder 混合历史与交互。
