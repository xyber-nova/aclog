## Purpose

为 `aclog` 提供一个统一的、actor-backed 的本地 `jj` 仓库访问层，让上层 workflow 通过同一套语义接口完成工作区校验、历史读取、revision 解析、文件跟踪判断和记录写入，而不需要显式区分 `jj-lib` 与 `jj` CLI。

## Requirements

### Requirement: 系统必须为应用层提供统一的仓库访问服务
系统 MUST 为应用层 workflow 提供一组统一的仓库语义接口，使调用方能够完成工作区校验、历史读取、revision 解析、文件跟踪判断和记录写入，而不需要显式区分 `jj-lib` 与 `jj` CLI。

#### Scenario: 应用层通过单一仓库抽象访问 `jj`
- **WHEN** `sync`、`record`、浏览或统计 workflow 需要读取或修改仓库状态
- **THEN** 调用方必须通过统一的仓库访问服务发起请求
- **AND** 调用方不得自行分支决定某个请求应直接走 `jj-lib` 还是 `jj` CLI

#### Scenario: 统一仓库服务覆盖现有核心用例
- **WHEN** 应用层需要完成工作区校验、变更检测、记录索引加载、revset 解析、tracked file 判断、创建 commit 或重写 commit 描述
- **THEN** 系统必须能够通过统一仓库服务提供这些能力

### Requirement: live 仓库访问必须通过 actor 串行化
系统的 live 仓库实现 MUST 通过单个 actor 在一次 CLI 命令内串行处理仓库请求，以保证 `jj` 读写顺序可预测并避免后续复杂功能中的隐式竞态。

#### Scenario: 同一命令中的写后读可见最新状态
- **WHEN** 某条 workflow 在同一次命令执行中先执行写操作，再执行后续读取
- **THEN** 后续读取必须观察到该写操作已经生效后的仓库状态

#### Scenario: 排队的仓库请求按顺序执行
- **WHEN** 同一次命令中存在多个连续发出的仓库请求
- **THEN** actor 必须按照请求进入队列的顺序依次处理这些操作
- **AND** 系统不得并发执行多个 live `jj` 操作而破坏顺序语义

#### Scenario: 同一进程中的同一工作区复用唯一 live actor
- **WHEN** 同一个 `aclog` 进程内多次请求同一个工作区的 live 仓库 handle
- **THEN** 系统必须复用同一个 live actor
- **AND** 系统不得为同一工作区并行创建多个 live actor 来处理仓库请求

### Requirement: actor 必须保持工作区级作用域，并在进程内保持唯一
仓库 actor MUST 与目标工作区绑定，并在同一个 `aclog` 进程内对该工作区保持唯一，而不得升级为跨进程共享服务。

#### Scenario: 一次 CLI 调用创建一个 live actor
- **WHEN** 用户执行任意依赖仓库访问的 `aclog` 命令
- **THEN** 系统必须为该次命令和目标工作区创建一个 live 仓库 actor
- **AND** 该命令中的仓库请求必须共享这一 actor

#### Scenario: 同一工作区不会在进程内生成第二个 live actor
- **WHEN** 该工作区的 live actor 已经存在
- **THEN** 后续请求必须返回指向同一 actor 的 handle
- **AND** 系统不得因为重复构造依赖对象而启动第二个 actor 线程

#### Scenario: 命令结束后 actor 一并退出
- **WHEN** CLI 命令正常结束或异常退出
- **THEN** 与该命令关联的 live 仓库 actor 必须随之结束
- **AND** 系统不得要求长期驻留的后台仓库服务

### Requirement: 读写后端选择必须保持在仓库层内部
系统 MUST 继续保持“`jj-lib` 优先负责只读、`jj` CLI 负责写操作”的项目约束，但这些后端选择只能存在于仓库层内部，而不得泄漏为调用方需要理解的接口差异。

#### Scenario: 读取请求优先使用 `jj-lib`
- **WHEN** 统一仓库服务处理工作区加载、历史遍历、快照、diff、revset 或 commit 元数据读取
- **THEN** live 实现必须优先使用 `jj-lib`

#### Scenario: 写入请求使用 `jj` CLI
- **WHEN** 统一仓库服务处理创建 commit、rewrite / describe 或其他改变仓库状态的动作
- **THEN** live 实现必须使用 `jj` CLI

#### Scenario: 调用方不能依赖后端实现细节
- **WHEN** 应用层使用统一仓库服务
- **THEN** 调用方不得假设某个接口背后固定对应 `jj-lib` API 或某条 `jj` shell 命令
- **AND** 后端实现替换不得要求 workflow 代码改动调用语义

### Requirement: actor-backed live 实现必须对测试保持可替换
系统 MUST 保持仓库访问边界可替换，使离线测试和 workflow 测试能够继续通过 fake 仓库实现运行，而不需要启动真实 actor 或真实 `jj`。

#### Scenario: workflow 测试使用 fake 仓库
- **WHEN** 自动化测试验证 `sync`、`record`、浏览或统计 workflow
- **THEN** 测试必须能够提供一个 fake 仓库实现来替代 live actor
- **AND** 测试不得被迫启动真实 `jj` actor runtime 才能验证业务逻辑

#### Scenario: actor 行为单独测试
- **WHEN** 自动化测试需要验证 FIFO 顺序、响应传递或写后读可见性
- **THEN** 系统可以为 live actor 增加针对性的单元测试或集成测试
- **AND** 这些测试必须与 workflow fake 测试分层
