## Context

`aclog` 当前已经有多个成熟的工作流入口：`sync` 负责当前工作副本记账，`stats` 负责历史统计与复习，`record browse` 负责浏览时间线，`record list` 负责非交互列表输出。真实终端 UI 也已经统一收口到 `src/ui/terminal/`，但顶层 CLI 仍然要求用户先记住子命令，再分别进入对应页面。

这次变更会同时触及 CLI 顶层解析、应用层 workflow 编排、终端首页交互与回归测试。设计需要保证三件事：

- 无参 `aclog` 可以自然地成为主入口，但不破坏 `--help` 和既有子命令的可预期性。
- 首页只是导航壳层，不重新定义 `sync`、`stats`、`browse` 的边界，也不把 UI 逻辑塞回 app 之外。
- 首页摘要必须只依赖本地可读数据，不能因为缺少远端凭据、标签字典或网络而失去可用性。

## Goals / Non-Goals

**Goals:**
- 让 `aclog` 无参时直接进入全局训练工作台。
- 为首页提供稳定的导航动作：恢复/开始 `sync`、打开 `stats`、打开文件/题目浏览、查看 `record list` 快照。
- 让首页复用现有统一终端主题、阅读结构和键位约定。
- 让首页摘要建立在本地 `jj` 历史和 `.aclog/sync-session.toml` 之上，不引入新的事实源。
- 保留所有现有子命令和非交互输出契约不变。

**Non-Goals:**
- 不在本次改动中引入 WebUI、常驻后台或新的本地数据库。
- 不把 `sync`、`stats`、`browse` 合并成一个共享的大状态机。
- 不把首页扩展成 onboarding / 教学系统，也不替代 `aclog init`。
- 不改变 `record list` 的 CLI 输出格式或 JSON 语义。

## Decisions

### 1. 顶层 CLI 改为“可选子命令 + 无参进入首页”
`src/cli.rs` 中的顶层 `command` 改为 `Option<Commands>`。当用户执行无参 `aclog` 时，直接以当前目录作为工作区进入新的首页 workflow；当用户提供任一现有子命令时，继续沿用原有分发逻辑；`aclog --help` 与 `aclog help` 继续由 clap 提供完整帮助。

这样可以把“训练工作台”变成默认主路径，同时不破坏脚本、文档和老用户对现有子命令的认知。

备选方案：
- 新增 `aclog home` / `aclog hub` 子命令。实现更保守，但会让最常用入口多打一层名字，违背“无参即主入口”的产品方向。
- 保持无参打印帮助，并在帮助里推荐首页。迁移成本更低，但无法真正把工作台提升为默认路径。

### 2. 首页 workflow 保持在 app 层，由 UI 返回动作枚举
新增独立的 `app::home` workflow，由它负责收集本地摘要、调用 UI 打开首页、接收用户动作，再分发到现有 `sync` / `stats` / `browser` / `record list` 逻辑。`UserInterface` 增加首页入口方法，返回类似 `HomeAction` 的动作枚举，而不是让终端 UI 直接调用其他 app workflow。

这样可以维持既有的“app 编排业务、UI 只展示和回传选择”的边界，也能让 fake UI 和 workflow 测试继续有清晰替身点。

备选方案：
- 让 `src/ui/terminal/home.rs` 直接调用 `run_sync` / `run_stats`。这样耦合更强，会把业务编排反向泄漏进 UI 层。
- 把首页塞进 `src/cli.rs` 直接分发。短期更省文件，但首页一旦有摘要逻辑和动作循环，CLI 层会迅速变成 workflow 容器。

### 3. 首页只消费本地可读摘要，不依赖配置校验或远端数据
首页摘要只使用这些本地来源：

- `AclogPaths`：解析工作区路径与 `.aclog/` 文件位置
- `RecordIndex`：读取本地 `solve(...)` 历史，生成总题数、总记录数、最近训练和 provider 分布等摘要
- `.aclog/sync-session.toml`：读取未完成批次，展示是否可恢复、待处理项数量和创建时间
- `RecordIndex::current_by_file()` + tracked file 判断：在首页内生成结构化的记录列表行模型，再交给首页 TUI 渲染只读快照

首页不调用题目元数据 API、不加载标签字典、不要求 `luogu_uid` / `cookie` 已配置。这样无参入口可以保持“离线、快速、低前置条件”，同时避免把 CLI 纯文本 renderer 直接混进 TUI 页面。

备选方案：
- 复用 `stats::run` 的完整摘要管线。这样能拿到更丰富的分布统计，但会把配置校验与 provider-specific 辅助数据加载一并带进首页，启动条件过重。
- 为首页引入新的缓存文件。会违背当前 “`jj` 历史即事实源” 和“`.aclog/` 只存配置/缓存/会话”的原则。

### 4. 首页通过“退出首页 -> 运行子工作流 -> 返回首页”实现导航
首页自身作为独立的终端工作台运行一次，UI 返回用户选择的动作后退出 alternate screen。随后 app 层调用既有 workflow；对应 workflow 结束后，再重新打开首页。这样首页不需要在一个终端生命周期内嵌套 `sync` / `stats` / `browse` 的状态机，也不需要把多个页面合并成共享路由系统。

对 `record list`，首页不直接打印到全局 stdout 后结束，而是在首页家族中提供只读快照页：app 层先构造与 `record list` 同口径的结构化行数据，再由终端 UI 用专门的表格布局负责查看和返回。

备选方案：
- 在单一 TUI 内嵌套所有页面。用户体验更像“应用”，但会把当前几个彼此独立的 workflow 全部卷进一个总状态机，复杂度过高。
- 让 `record list` 从首页触发后直接打印到 shell。实现最省，但会打断“从首页进入、查看、再返回首页”的主路径。

### 5. 首页纳入统一 terminal workbench 设计语言，但不抢占已有快捷键语义
首页布局沿用现有工作台的“上下文 + 主入口列表 + 摘要/详情 + 操作提示”结构，支持 `Enter`、`Esc`、`q`、`?`、`j/k`。入口项按工作流组织，而不是按技术模块命名，例如“恢复 sync 批次”“开始 sync”“训练统计”“文件浏览”“题目浏览”“记录列表”。

这样首页能与既有 `sync` / `stats` / `browser` 同属于一个终端家族，又不会要求用户为首屏再学习另一套交互规则。

备选方案：
- 做成纯命令 palette。更极简，但对“摘要 + 恢复状态”这类首页信息承载不足。
- 复用 browser 的列表结构但不显示摘要。实现更快，但会弱化首页作为导航壳层的价值。

## Risks / Trade-offs

- [Risk] 无参 `aclog` 从“帮助优先”切到“首页优先”后，首次使用者可能更晚发现完整命令集。
  → Mitigation: 保留 `--help` / `help`，并在首页帮助区显式提示子命令仍可直达。

- [Risk] 首页如果直接依赖 `load_config` 或远端辅助数据，会让无参入口在未配置或离线场景下失效。
  → Mitigation: 首页摘要严格限定为本地只读数据，不走远端与凭据校验路径。

- [Risk] `record list` 是非交互输出，将它并入首页容易造成重复实现或 UI/CLI 语义漂移。
  → Mitigation: 复用 `record list` 的记录解释口径，但为首页单独维护结构化行模型和 TUI 表格渲染；首页中的列表查看器仍定义为“只读快照页”，不改 CLI 输出协议。

- [Risk] 首页反复退出并重新打开 terminal screen 可能带来轻微闪屏。
  → Mitigation: 优先保持 workflow 分离与实现清晰度；若后续体验明显受影响，再单独评估共享路由壳层。

- [Risk] 用户可能期待首页顺手承担 `init` / onboarding。
  → Mitigation: 本次明确不扩 onboarding；若工作区不合法，继续给出清晰错误并提示使用 `aclog init`。
