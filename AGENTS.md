# ACLog 代理说明

## 项目概况

`aclog` 是一个 Rust CLI，用来在本地 `jj` 工作区中记录 OI / 算法训练过程。

当前已有的用户命令：

- `aclog init`
  - 初始化 `.aclog/`
  - 生成默认 `config.toml`
  - 初始化同目录 `jj` 仓库
- `aclog sync`
  - 使用 `jj` 检测题目文件变更
  - 支持 `--dry-run` 只预览当前批次，不创建 commit
  - 支持 `--resume` 恢复 `.aclog/sync-session.toml` 中未完成的批次
  - 支持 `--rebuild` 丢弃已有恢复状态并按当前工作区重建批次
  - 从文件名中提取题号，例如 `P1000.cpp`、`CF1234A.cpp`
  - 拉取洛谷题目元数据和用户提交记录
  - 默认先进入批次预览页，再进入单文件详情页
  - 会在详情页里让用户显式选择：
    - 绑定某条提交记录
    - 标记为 `chore`
    - 跳过
    - 对已删除文件记为 `remove`
  - 会在提交前展示可恢复的一致性告警，例如重复绑定、题号不一致
  - 每个文件生成一个 `jj` commit
- `aclog stats`
  - 汇总历史 `solve` 记录并展示统计界面
  - 统计中按算法标签聚合（非算法标签会过滤）
  - 支持时间窗口统计与 review 候选输出
- `aclog record bind <file> [--submission-id <id>]`
  - 为已跟踪的解法文件新建一条 `solve` 记录 commit
  - 未给 `--submission-id` 时进入 Selector TUI 选择提交
- `aclog record rebind <file> [--record-rev <rev>] [--submission-id <id>]`
  - 只重写已有 `solve` 记录的提交信息（不改题目文件内容）
  - `--record-rev` 缺失时进入 Selector TUI 先选要重写的记录
  - `--submission-id` 缺失时进入 Selector TUI 再选新的提交
- `aclog record list`
  - 按文件列出当前工作区已记录的解法
  - 仅显示当前仍被 `jj` 跟踪的文件
  - 支持过滤参数和结构化输出
- `aclog record browse`
  - 打开记录浏览工作台
  - 支持 `files` / `problems` 两种根视角
  - 支持按题号、文件名、结果、难度、标签、时间窗口过滤
  - 支持从列表进入文件或题目的时间线，再查看完整记录详情
- `aclog record show <file> [--record-rev <rev>]`
  - 查看某个文件当前记录或指定历史记录的完整详情
- `aclog record edit <file> [--record-rev <rev>] [--note ...] [--mistakes ...] ...`
  - 只重写已有 `solve` 记录里的训练字段
  - 不改题目文件内容

## 技术栈

- Rust 2024
- `jj-lib`：仓库读取与变更分析
- `ratatui` + `crossterm`：TUI
- `reqwest` + `tokio`：洛谷 HTTP 请求
- `serde` + `toml`：配置与缓存
- `tracing` + `color-eyre`：日志与错误处理

## 目录职责

- `src/lib.rs`
  - library 入口，供集成测试和二进制入口共享
- `src/main.rs`
  - 二进制入口
- `src/cli.rs`
  - 命令解析与 CLI 分发
- `src/app/`
  - 应用层 workflow 编排与依赖注入接口
- `src/config.rs`
  - `.aclog` 路径解析与配置读取
- `src/api/`
  - 洛谷 HTTP 客户端、元数据与提交记录读取、缓存逻辑
- `src/domain/`
  - 领域模型与统计聚合
- `src/commit_format.rs`
  - `solve(...)` / `remove(...)` commit message 构造与解析
- `src/problem.rs`
  - 从文件名提取题号
- `src/tui.rs`
  - 提交记录选择界面
- `src/ui/`
  - 交互抽象与 TUI 适配
- `src/vcs/`
  - `jj` 相关读写能力
- `src/models.rs`
  - 向后兼容 re-export

## `jj` 读写分离约束

- 默认优先使用 `jj-lib`。
- `jj-lib` 负责只读能力：
  - 工作区与仓库加载
  - 工作副本快照
  - 历史遍历
  - diff / tree / commit 元数据读取
- `jj` CLI 负责写操作：
  - 创建 commit
  - rewrite / describe
  - 其他会改变仓库状态的动作
- 只有在 `jj-lib` 难以稳定覆盖、而且确实不可避免时，才允许为只读场景调用 `jj` CLI。
- 设计新功能时，先判断是否能用 `jj-lib` 完成只读部分；不要因为 CLI 更顺手就直接把读路径也实现成 shell 调用。

## `jj` as database 原则

- 当前项目把本地 `jj` 历史视为训练记录的事实源。
- `solve(...)` / `chore(...)` / `remove(...)` commit 描述不是普通说明文字，而是带稳定结构的记录载体。
- `.aclog/` 下的内容只承担配置、缓存和会话状态职责，不承担训练记录真值职责：
  - `config.toml`：用户配置
  - `problems/*.toml`：题目元数据缓存
  - `luogu-mappings.toml` / `luogu-tags.toml`：映射与标签缓存
  - 未来新增的 sync session 文件：流程恢复状态
- 任何“当前记录状态”“题目时间线”“统计结果”“复习建议”都应从 `jj` 历史推导，而不是另建一份可写的数据库副本。
- `record rebind` / `record edit` 的语义是重写已有记录，使该 commit 自身继续承担事实源；不要通过额外 append 一条“修正记录”来模拟更新。

## 记录提交信息协议

当前主要有三类 commit message：

- `solve(<problem-id>): <title>`
  - 正式做题记录，是主要数据载体
- `chore(<problem-id>): 本地修改`
  - 用户显式标记为非正式做题记录的本地维护动作
- `remove(<problem-id>): 删除题解文件`
  - 题解文件删除记录

其中 `solve(...)` 的正文采用“首行 + 英文 Key: Value 字段”的结构化格式。当前字段组织为：

- 基础提交信息
  - `Verdict`
  - `Score`
  - `Time`
  - `Memory`
  - `Submission-ID`
  - `Submission-Time`
- 题目上下文
  - `Tags`
  - `Difficulty`
  - `Source`
  - `File`
- 训练字段
  - `Note`
  - `Mistakes`
  - `Insight`
  - `Confidence`
  - `Source-Kind`
  - `Time-Spent`

约束：

- commit 描述里的结构化字段标签当前保持英文，视为记录协议的一部分，不随界面语言本地化。
- 新生成记录应使用当前协议字段名。
- 解析层可以兼容历史上已经存在的中英文标签别名，但不要再引入新的随意变体。
- 如果以后扩字段，优先追加新字段；不要破坏已存在字段的语义。

## 全图统一解析语义

- 除非某个功能明确只面向“当前工作副本状态”，否则默认按整个本地 `jj` 图解析历史，而不是只看当前 head 所在路径。
- 当前实现里，全图历史读取统一基于 `all()` revset。
- 因此：
  - `record list`
  - `record show`
  - `record edit`
  - `record rebind`
  - `stats`
  - review / suggestion
  都应建立在“全图统一解析”的语义上。

在多叉树场景下，当前规则是：

- 同一文件或同一题的候选记录可以来自不同叉上的 commit。
- “当前状态”不是按当前 head 路径挑选，而是按记录语义挑选：
  - 优先比较 `Submission-Time`
  - 若缺失，再按解析顺序 / `source_order`
- 因此系统当前更接近“全仓库训练事实库”，而不是“当前栈视图”。

唯一明确的例外是 `sync`：

- `sync` 只面向当前工作副本
- 它读取的是当前 working-copy commit 与其父提交之间的 diff
- 它关心的是“现在这次工作区改动该如何记账”，不是全图历史聚合

设计新功能时，必须先明确它属于哪一种语义：

- `全图历史派生`
  - 浏览、统计、建议、记录查看、记录维护
- `当前工作副本派生`
  - sync、tracked file 判断、工作区变更检测

不要把这两类语义混在一起。

## 工作区文件

用户工作区内会使用这些文件：

- `.aclog/config.toml`
  - 用户配置
- `.aclog/problems/<problem_id>.toml`
  - 单题题目元数据缓存
- `.aclog/luogu-mappings.toml`
  - 从 `/_lfe/config` 拉取的洛谷共享映射缓存
- `.aclog/sync-session.toml`
  - 未完成 `sync` 批次的恢复状态

当前缓存 TTL 现在分为三个字段：

- `problem_metadata_ttl_days`：题目元数据缓存
- `luogu_mappings_ttl_days`：洛谷共享映射缓存
- `luogu_tags_ttl_days`：洛谷标签字典缓存

`metadata_ttl_days` 仍可作为旧配置回退值。

## 洛谷 API 索引

统一参考：

- `https://0f-0b.github.io/luogu-api-docs/`
- `https://0f-0b.github.io/luogu-api-docs/misc`

当前项目实际依赖的接口主要是：

- `/_lfe/config`
  - 用于 `recordStatus` 和 `problemDifficulty` 映射
- `/_lfe/tags`
  - 用于标签字典
- `/problem/{pid}?_contentOnly=1`
  - 用于题目元数据
- `/record/list?pid={pid}&user={uid}&_contentOnly=1`
  - 用于用户提交记录

关键备注：

- `problem` 和 `record/list` 请求当前都需要带 `x-lentille-request: content-only`
- 状态码与难度显示名应优先来自 `/_lfe/config`
- 详细字段说明优先看文档页，不要把整套接口说明复制进仓库文档

## 已验证的真实返回字段

从真实 `record/list` 返回中，已确认记录里常见这些字段：

- `id`
- `status`
- `score`
- `time`
- `memory`
- `submitTime`
- `user.name`
- `user.uid`
- `problem.pid`
- `problem.title`
- `problem.difficulty`

重要说明：

- `status` 可能只有数字，没有文本状态名
- 这种情况下必须通过 `/_lfe/config -> recordStatus` 映射显示名
- 已验证示例：
  - `status = 14` -> `Unaccepted`

## 当前 TUI 行为

- `sync`
  - 先展示批次预览页，列出文件、题号、类型、状态、提交数、默认候选
  - 预览页按键：
    - `↑/↓`：移动
    - `Enter`：进入当前文件详情
    - `Esc`：暂停并保留 `.aclog/sync-session.toml`
  - 有未完成批次时，恢复页按键：
    - `r`：恢复
    - `n`：丢弃旧批次并重建
  - 详情页按键：
    - `Enter`：选择当前提交记录，或确认删除
    - `c`：标记为 `chore`
    - `Esc`：跳过当前文件
  - 即使没有提交记录，也会进入空状态界面，而不是自动生成 `chore`
- `record browse`
  - 支持文件视角和题目视角
  - `Tab` 在两种根视角间切换
  - `Enter` 进入文件或题目的时间线
  - 时间线页按 `b` 返回
- `stats`
  - 默认打开统计页
  - `r` 进入 review 候选
  - `Enter` 从建议项钻取到浏览工作台
  - `f` / `p` 直接进入文件 / 题目浏览
- 结果列颜色：
  - `AC`：绿色
  - `WA`：红色
  - 其他结果：默认颜色

## CLI 中的交互模式边界

- `aclog` 是 CLI 工具；TUI 是这个 CLI 内部的一种交互式终端界面，不是独立于 CLI 的另一套系统。
- 命令层负责：
  - 命令解析与参数校验
  - 工作区 / 配置 / 路径 / `jj` 状态检查
  - API 数据拉取与本地缓存读取
  - 业务流程编排
  - `jj` commit / rewrite 等仓库操作
  - 非交互输出，例如列表和报错
- 交互式终端界面负责：
  - 把 CLI 已经准备好的候选集合展示给用户
  - 返回用户选中的项
- 交互式终端界面不负责：
  - 直接访问 API
  - 直接读取或修改 `jj`
  - 决定命令流程
  - 隐式补齐业务判断
- 任何通过交互式终端界面完成的关键选择，都必须存在等价的非交互 CLI 表达方式；不要把能力做成界面独占。
- 设计新命令时，先定义清楚命令语义和非交互输入 / 输出，再决定是否补充 TUI 作为默认选择器。

## 实现约束

- 不要重新引入洛谷状态码和难度的硬编码映射，只要 `/_lfe/config` 能提供，就应优先使用
- 如果配置映射缺失，状态显示原始编号形式，例如 `Status-14`，不要猜测含义
- `chore` 只能来自用户显式操作，不能作为隐式默认行为
- 继续保持：
  - 一个变更文件
  - 一次选择
  - 一个 commit

## 测试约定

- 默认测试套件必须可离线运行：
  - `cargo test` 不依赖真实 Luogu 网络
  - `cargo test` 不依赖真实 Luogu 账号凭据
- 自动化测试分三层：
  - 模块内单元测试
  - 使用 fake 依赖的 workflow 集成测试
  - 使用真实临时 `jj` 工作区的集成测试
- `tests/support/` 负责共享 fake provider、fake repo、fake UI、输出捕获器和工作区 fixture；新增测试优先复用这些工具。
- 非交互 CLI 输出优先设计为“纯渲染函数 + 输出分发”，避免测试依赖全局 stdout 捕获。
- 默认 CI 门禁执行：
  - `cargo fmt --check`
  - `cargo check`
  - `cargo test`
