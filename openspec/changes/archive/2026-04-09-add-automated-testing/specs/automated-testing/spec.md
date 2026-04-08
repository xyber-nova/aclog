## ADDED Requirements

### Requirement: 项目必须提供可在库层复用的应用入口
系统必须将核心模块暴露为 library crate，并允许自动化测试直接调用应用层 workflow，而不必只能通过二进制子进程黑盒触发。

#### Scenario: 集成测试调用应用层 workflow
- **WHEN** 自动化测试需要验证 `sync`、`record bind`、`record rebind`、`record list` 或 `stats` 的 workflow
- **THEN** 测试必须能够通过 library crate 直接调用相应应用入口
- **AND** 测试不得被迫只能通过启动 CLI 子进程完成验证

### Requirement: 应用层必须支持可替换的外部依赖
应用层 workflow 必须通过显式接口依赖题目/提交提供者、仓库读写能力和非交互输出能力，以便自动化测试在无真实 Luogu 网络、无真实 stdout 的条件下稳定执行。

#### Scenario: workflow 使用 fake 题目与提交提供者
- **WHEN** 自动化测试为某个 workflow 注入 fake 题目元数据和 submission 数据
- **THEN** workflow 必须使用这些 fake 数据完成业务决策
- **AND** workflow 不得主动访问真实 Luogu 网络

#### Scenario: workflow 使用 fake 仓库依赖
- **WHEN** 自动化测试为某个 workflow 注入 fake 仓库读写能力
- **THEN** workflow 必须把工作区查询、历史读取、提交创建和描述重写委托给该依赖
- **AND** 测试必须能够断言这些仓库副作用请求的参数

#### Scenario: record list 使用可替换输出目标
- **WHEN** 自动化测试执行 `record list` workflow
- **THEN** workflow 必须能够把输出写入可替换的输出接口
- **AND** 测试不得依赖全局 stdout 捕获才能验证结果

### Requirement: 默认测试套件必须在离线环境下通过
项目默认自动化测试套件必须能够在没有 Luogu 凭据、没有真实网络访问的环境中通过，并仅对本地可用的 `jj` 集成测试提出依赖。

#### Scenario: 在无 Luogu 配置的环境中运行默认测试
- **WHEN** 开发者或 CI 在未提供 `luogu_cookie`、`luogu_uid` 且不访问外网的环境中执行默认测试命令
- **THEN** 默认测试套件必须能够通过
- **AND** 系统不得要求真实 Luogu 账号信息作为默认测试前提

### Requirement: 项目必须提供 workflow 级自动化测试覆盖
项目必须为核心 workflow 提供 deterministic 的应用层自动化测试，覆盖主要分支和关键副作用，而不依赖真实终端交互。

#### Scenario: sync 覆盖四种选择结果
- **WHEN** 自动化测试执行 `sync` workflow
- **THEN** 测试必须能够覆盖 `submission`、`chore`、`delete` 和 `skip` 四种用户结果
- **AND** 测试必须能够断言对应 commit 计划或跳过行为

#### Scenario: record bind 与 record rebind 覆盖交互和非交互路径
- **WHEN** 自动化测试执行 `record bind` 或 `record rebind` workflow
- **THEN** 测试必须能够覆盖 CLI 直接指定参数和需要交互补全两类路径
- **AND** 测试必须能够验证非法 submission、非法 record revision 或题号不匹配时的报错行为

#### Scenario: stats 覆盖聚合与展示输入
- **WHEN** 自动化测试执行 `stats` workflow
- **THEN** 测试必须能够验证本地 solve 历史解析、算法标签过滤和传递给 UI 的 summary 内容

### Requirement: 项目必须保留真实 jj 集成测试
项目必须提供少量基于真实临时 `jj` 工作区的集成测试，用于验证 fake 无法完全证明的仓库联动行为。

#### Scenario: 真实 jj 集成测试验证仓库写操作
- **WHEN** 集成测试在临时目录初始化真实 `jj` 工作区并执行提交创建或描述重写
- **THEN** 测试必须验证这些操作在真实仓库中的结果可被后续历史读取观察到

#### Scenario: 真实 jj 集成测试验证工作区读取行为
- **WHEN** 集成测试在真实 `jj` 工作区中创建、修改或删除题目文件
- **THEN** 测试必须能够验证变更文件识别和 tracked file 判断行为

### Requirement: 项目必须提供基础 CI 测试门禁
项目必须提供默认 Linux CI workflow，自动执行格式检查、编译检查和测试，以确保自动化测试方案真正成为团队门禁。

#### Scenario: CI 运行基础门禁
- **WHEN** 代码变更触发默认 CI workflow
- **THEN** 系统必须执行 `cargo fmt --check`
- **AND** 系统必须执行 `cargo check`
- **AND** 系统必须在安装 `jj` 后执行 `cargo test`
