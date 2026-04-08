## Why

`aclog` 目前已经有一批单元测试，但测试主要集中在模块内部纯逻辑，缺少对应用层 workflow、真实 `jj` 工作区联动和 CLI 门禁的自动化覆盖。随着后续继续迭代命令、缓存和交互流程，如果现在不补齐稳定的自动化测试方案，回归风险和重构成本都会明显上升。

## What Changes

- 把项目调整为 `lib + bin` 结构，使应用层 workflow 能被集成测试直接调用，而不必只能通过二进制黑盒触发。
- 为应用层补齐可注入的测试依赖接口，覆盖题目/提交提供者、仓库读写能力和非交互输出能力，允许在测试中使用 deterministic fake/stub。
- 新增 `tests/` 集成测试体系，覆盖 `sync`、`record bind`、`record rebind`、`record list`、`stats` 等核心 workflow，并补充少量真实 `jj` 工作区集成测试。
- 为 `record list` 提供纯渲染输出函数，使输出格式既可单测，也可用于 CLI 黑盒断言。
- 新增 Linux CI workflow，默认执行 `cargo fmt --check`、`cargo check` 和 `cargo test`，形成基础自动化门禁。

## Capabilities

### New Capabilities
- `automated-testing`: 为项目提供本地稳定、可重复执行的自动化测试结构、workflow 级测试能力和基础 CI 门禁。

### Modified Capabilities
- None.

## Impact

- 受影响代码主要在 crate 入口结构、`app` workflow 编排层、`api` / `vcs` 外部依赖调用边界、非交互输出路径和测试目录组织。
- 新增对开发流程可见的接口类型，例如应用层依赖注入 trait 和 `record list` 渲染入口。
- 新增 `tests/` 集成测试目录与 `.github/workflows/ci.yml`。
- 不改变现有用户命令语义，不引入真实 Luogu 网络依赖进入默认测试套件。
