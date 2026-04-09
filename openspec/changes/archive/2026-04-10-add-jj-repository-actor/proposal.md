## Why

`aclog` 当前已经刻意把 `jj-lib` 用于只读、把 `jj` CLI 用于写操作，但上层 workflow 仍然直接组合低层仓库操作，导致“怎么访问 `jj`”泄漏到了应用层。随着 `jj-history-as-database` 语义已经正式成型，现在正适合把仓库边界收紧成统一入口，提前消除未来在复杂交互和后续重构中引入时序 bug 的风险。

## What Changes

- 引入一个轻量级、命令级生命周期的 actor-backed 仓库服务，在单次 CLI 命令内串行化所有 `jj` 访问。
- 以更高层的仓库语义接口替换当前偏底层的 `RepoGateway` 操作集合，让上层 workflow 不再显式区分 `jj-lib` 与 `jj` CLI。
- 保留内部读写分离：只读路径继续优先使用 `jj-lib`，写路径继续使用 `jj` CLI，但这些实现细节只能存在于仓库层内部。
- 修改 `jj-history-as-database` 约束，要求历史派生与记录维护都通过统一仓库访问层完成，并保证命令内仓库操作遵守串行顺序。
- 保持现有 CLI 命令语义、交互流程和用户可见输出不变；这次 change 主要是架构重构与约束升级，不引入新的用户命令。

## Capabilities

### New Capabilities
- `jj-repository-actor`: 提供一个 actor-backed 的统一仓库访问服务，在单次命令内串行化 `jj` 操作并向上层隐藏 `jj-lib`/`jj` CLI 选择细节

### Modified Capabilities
- `jj-history-as-database`: 增加“统一仓库访问层”和“命令内仓库操作顺序保证”的正式要求

## Impact

- 受影响代码：
  - `src/app/deps.rs`
  - `src/app/mod.rs`
  - `src/app/support.rs`
  - `src/vcs/`
  - `tests/`
  - `tests/support/`
- 受影响接口：
  - 仓库访问 trait 将从偏底层操作重构为更高层的语义接口
  - `LiveDeps` 将从零大小 live adapter 改为持有 actor handle 的命令级依赖对象
- 受影响依赖：
  - `Cargo.toml` 中的 Tokio feature 需要补充 `sync`
- 兼容性：
  - 不改变现有 CLI 命令面
  - 不改变 `jj-lib` 读 / `jj` CLI 写的项目约束
