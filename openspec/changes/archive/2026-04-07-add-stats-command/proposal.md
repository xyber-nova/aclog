## Why

`aclog` 目前只能在 `sync` 过程中逐题绑定记录并生成 commit，但用户无法直接查看这个工作区里已经记录了多少题、通过情况如何、难度分布如何，以及训练内容主要集中在哪些算法标签上。补上一个本地统计界面可以让用户快速了解训练进度，并复用当前已经沉淀在 `jj` 历史中的做题信息。

## What Changes

- 新增独立命令 `aclog stats`，用于展示当前工作区的做题统计界面。
- 从本地 `jj` 历史中的 `solve(...)` commit 提取做题记录，生成统计摘要；为识别算法标签，允许复用或刷新 `/_lfe/tags` 字典缓存，但不抓取远端全量做题记录。
- 提供只读 TUI 概览页，展示唯一题目数、`solve` 记录数、AC/非 AC 情况、按结果分布、按难度分布和按算法标签分布。
- 将 `/_lfe/tags` 的标签缓存结构扩展为包含标签类型信息，并在统计阶段过滤非算法标签。
- 明确同一道题多次 `solve(...)` 时，同时支持“唯一题目口径”和“全部记录口径”的统计展示。

## Capabilities

### New Capabilities
- `stats-command`: 为当前工作区提供基于本地 `jj` 历史的做题统计命令和终端统计界面。

### Modified Capabilities
- None.

## Impact

- 受影响代码主要在 CLI 命令分发、`jj` 历史读取、洛谷标签缓存、统计聚合模型与 TUI 渲染。
- 新增一个公开命令接口：`aclog stats [--workspace <path>]`。
- 新增 `.aclog/luogu-tags.toml` 缓存文件，用于保存标签名称、类型和父标签信息。
- 不修改现有 `sync` 提交流程与 commit message 格式。
