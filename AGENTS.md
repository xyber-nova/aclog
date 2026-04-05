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
  - 从文件名中提取题号，例如 `P1000.cpp`、`CF1234A.cpp`
  - 拉取洛谷题目元数据和用户提交记录
  - 打开 TUI，让用户显式选择：
    - 绑定某条提交记录
    - 标记为 `chore`
    - 跳过
  - 每个文件生成一个 `jj` commit
- `aclog stats`
  - 汇总历史 `solve` 记录并展示统计界面
  - 统计中按算法标签聚合（非算法标签会过滤）
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

## 技术栈

- Rust 2024
- `jj-lib`：仓库读取与变更分析
- `ratatui` + `crossterm`：TUI
- `reqwest` + `tokio`：洛谷 HTTP 请求
- `serde` + `toml`：配置与缓存
- `tracing` + `color-eyre`：日志与错误处理

## 目录职责

- `src/main.rs`
  - 程序入口
- `src/cli.rs`
  - 命令解析与 `sync` 主流程
- `src/config.rs`
  - `.aclog` 路径解析与配置读取
- `src/api/`
  - 洛谷 HTTP 客户端、元数据与提交记录读取、缓存逻辑
- `src/problem.rs`
  - 从文件名提取题号
- `src/tui.rs`
  - 提交记录选择界面
- `src/vcs.rs`
  - `jj` 相关操作与 commit 创建
- `src/models.rs`
  - 数据模型与 commit message 拼装

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

## 工作区文件

用户工作区内会使用这些文件：

- `.aclog/config.toml`
  - 用户配置
- `.aclog/problems/<problem_id>.toml`
  - 单题题目元数据缓存
- `.aclog/luogu-mappings.toml`
  - 从 `/_lfe/config` 拉取的洛谷共享映射缓存

当前 `metadata_ttl_days` 同时用于：

- 题目元数据缓存
- 洛谷共享映射缓存

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

- 对每个变更题目文件都进入选择界面
- 即使没有提交记录，也会进入空状态界面，而不是自动生成 `chore`
- 按键：
  - `Enter`：选择当前提交记录
  - `c`：标记为 `chore`
  - `Esc`：跳过当前文件
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
