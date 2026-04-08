## Context

`aclog` 现在已经完成了 `app` / `ui` / `api` / `vcs` 的基础分层，并且现有测试主要覆盖纯逻辑模块，例如 commit message 解析、统计聚合、Luogu 字段映射和 TUI reducer。但应用层 workflow 仍然直接依赖真实 `api::*`、真实 `vcs::*` 和 stdout，这让 `sync`、`record bind`、`record rebind`、`record list`、`stats` 这些核心流程难以在无网络、无真实终端的条件下做稳定的自动化验证。

这次改动跨越 crate 入口结构、应用层依赖组织、测试目录布局和 CI 门禁，属于典型的跨模块工程能力建设。项目约束也比较明确：默认测试不能依赖真实 Luogu 网络，不能把完整 TUI 终端仿真作为主路径，同时需要保留当前 CLI 语义和 `jj` 真实联动能力。

## Goals / Non-Goals

**Goals:**
- 把项目调整为 `lib + bin` 结构，使集成测试可以直接调用应用层 workflow。
- 为应用层提供可注入的外部依赖接口，使 workflow 测试能在 deterministic fake/stub 上运行。
- 建立三层测试体系：模块内单元测试、应用层集成测试、真实 `jj` 工作区集成测试。
- 为 `record list` 提供纯渲染输出入口，避免测试依赖 stdout 捕获。
- 增加基础 Linux CI，默认执行格式、编译和测试门禁。

**Non-Goals:**
- 不把真实 Luogu 线上请求纳入默认测试套件。
- 不实现完整的 TUI 终端录制式 E2E 测试。
- 不在这轮承诺 Windows/macOS CI 支持。
- 不改变现有用户命令语义和 OpenSpec 已归档功能的行为合同。

## Decisions

### 1. crate 改为 `lib + bin`，让测试直接调用库入口

保留 `src/main.rs` 作为极薄的启动器，把模块声明和可复用入口迁到 `src/lib.rs`。这样集成测试既可以走 CLI 黑盒，也可以直接调用 `app::*` workflow，而不需要把每个场景都变成子进程黑盒测试。

选择这个方案，是因为当前大量逻辑已经沉淀在 `app` 层，直接复用会比重复拼接 CLI 命令更稳、更快，也更容易断言中间副作用。

备选方案：
- 只做二进制黑盒测试：覆盖真实，但对中间行为观察能力弱，且 fake 依赖很难接入。
- 继续把所有逻辑留在 bin crate：实现成本低，但测试扩展性差。

### 2. 应用层统一走依赖注入，而不是直接引用全局模块

在 `app` 层引入一组组合依赖接口：
- `ProblemProvider`：提供题目元数据、提交记录、算法标签集合。
- `RepoGateway`：封装工作区校验、历史读取、tracked file 判断、提交创建与描述重写。
- `OutputSink`：负责 CLI 文本输出，尤其是 `record list`。

生产实现继续委托给当前 `api::*`、`vcs::*` 和标准输出；测试实现则使用内存 fake，支持预置返回值和记录调用。`UserInterface` 继续作为交互层接口保留，不再让 workflow 自己直接碰 TUI。

选择这个方案，是因为应用层当前最大的不可测点不是逻辑本身，而是外部副作用绑定太死。统一抽象后，workflow 测试能专注于“输入 -> 决策 -> 副作用计划”。

备选方案：
- 只给 `api` 打 mock：不够，`jj` 和 stdout 仍会阻碍 workflow 测试。
- 直接在测试里 patch 全局函数：Rust 下不自然，也不利于长期维护。

### 3. 默认测试金字塔采用“deterministic 优先，真实集成兜底”

测试分成三层：
- 单元测试：继续覆盖纯函数和小型 reducer。
- 应用层集成测试：全部用 fake provider / fake repo / fake ui / fake output，默认不依赖网络和真实终端。
- 真实 `jj` 集成测试：只覆盖 fake 无法证明的仓库联动，例如真实 commit 创建、rewrite、tracked file 判定和变更识别。

选择这个方案，是为了让大多数测试保持快、稳、可重复，同时保留一小层真实仓库验证防止抽象层和真实 `jj` 行为脱节。

备选方案：
- 全部使用真实环境：维护成本高，测试容易波动。
- 全部使用 fake：速度快，但无法证明真实 `jj` 联动没偏差。

### 4. `record list` 改为“渲染字符串 + 输出”

把当前直接 `println!` 的路径改成两层：
- 渲染函数：接收 `FileRecordSummary` 列表并返回完整文本。
- 输出层：把文本写入 `OutputSink` 或 stdout。

这样既方便单元测试和应用层集成测试，也方便 CLI 黑盒只断言最终文本，而不需要依赖全局 stdout 捕获。

备选方案：
- 继续直接打印：实现最省，但单测不自然，输出断言容易变脆。

### 5. CI 首版只做 Linux 基础门禁

新增 GitHub Actions workflow，分为：
- `fmt`: `cargo fmt --check`
- `check`: `cargo check`
- `test`: 安装 `jj` 后执行 `cargo test`

首版只跑 Linux，因为当前实现直接依赖 `zsh` 和本地 `jj` 命令，先把最稳定的平台打通最划算。后续若需要跨平台，再单独处理 shell 和 `jj` 可用性。

备选方案：
- 一开始就做多平台矩阵：覆盖更广，但会把这轮测试方案拖进平台兼容问题。
- 不上 CI：本地可用，但无法形成团队门禁。

## Risks / Trade-offs

- [依赖注入改动会触碰较多 workflow 签名] → 先在 `app` 层引入组合依赖容器，保持 CLI 调用入口不变，减少外部扩散。
- [fake 实现与真实 `jj` 行为可能偏离] → 保留少量真实 `jj` 集成测试，专门覆盖仓库行为边界。
- [测试目录扩张后维护成本上升] → 统一 `tests/support` 工具箱，避免每个测试文件重复构造 fixture。
- [CI 引入后门禁失败会短期增加改动成本] → 首版门禁只放格式、编译、测试三项，不引入更重的 live smoke 或跨平台矩阵。
- [stdout/TUI 断言容易变脆] → 对非交互输出改为纯渲染函数；TUI 继续以 reducer 和 helper 级测试为主，不做终端像素级断言。
