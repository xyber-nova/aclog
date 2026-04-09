## Purpose

为 `aclog` 定义“本地 `jj` 历史即训练记录数据库”的基础语义，统一历史读取、记录维护和工作区缓存的职责边界，避免后续功能各自发明另一套记录真值来源。

## Requirements

### Requirement: 历史派生能力必须以本地 `jj` 历史为训练事实源
除非某个功能明确只面向当前工作副本状态，否则系统 MUST 以当前工作区本地 `jj` 历史中的结构化记录作为训练事实源，而不是以 `.aclog/*` 中的可写文件作为记录真值。

#### Scenario: `solve(...)` 是正式训练记录载体
- **WHEN** 系统需要读取做题记录、训练字段、题目时间线或统计输入
- **THEN** 系统必须从本地 `jj` 历史中的标准 `solve(...)` commit message 解析这些信息
- **AND** 系统必须把该 commit 自身视为正式训练记录载体

#### Scenario: `chore(...)` 与 `remove(...)` 不参与 solve 历史聚合
- **WHEN** 历史派生能力遍历本地 `jj` 图中的提交
- **THEN** 系统可以识别 `chore(...)` 与 `remove(...)` 作为协议内 commit 类型
- **AND** 系统不得把它们当作标准 `solve(...)` 记录计入记录列表、时间线当前状态、统计或复习建议主体数据

#### Scenario: 删除缓存不会抹掉训练历史
- **WHEN** `.aclog/problems/*.toml`、`luogu-mappings.toml`、`luogu-tags.toml` 或其他缓存文件被删除、过期或重建
- **THEN** 系统仍然必须能够从本地 `jj` 历史恢复已存在的训练记录事实
- **AND** 系统不得把这些缓存文件缺失解释为训练历史丢失

### Requirement: 全图历史派生必须默认读取整个本地 `jj` 图
对于浏览、统计、记录查看和记录维护等历史派生能力，系统 MUST 默认基于整个本地 `jj` 图解析历史，而不是只看当前 head 所在路径或当前栈视图。

#### Scenario: 全图读取使用统一历史范围
- **WHEN** 系统为历史派生能力读取工作区记录
- **THEN** 系统必须以整个本地 `jj` 图中的候选提交为读取范围
- **AND** 系统不得只因为某条记录不在当前 head 路径上就忽略它

#### Scenario: 当前状态优先按 `Submission-Time` 选取
- **WHEN** 同一文件或同一题目存在多条可识别的 `solve(...)` 记录
- **THEN** 系统必须优先使用 `Submission-Time` 较新的记录作为“当前状态”

#### Scenario: 缺失 `Submission-Time` 时回退到解析顺序
- **WHEN** 同一文件或同一题目的多条候选记录都缺失 `Submission-Time`
- **THEN** 系统必须回退到稳定的解析顺序 / `source_order` 选择当前状态
- **AND** 系统不得临时引入另一套未声明的“最新记录”判定规则

### Requirement: 历史派生能力必须共享统一读模型
`record list`、`record show`、`record edit`、`record rebind`、记录浏览工作台、`stats` 和复习建议等历史派生能力 MUST 建立在同一套 `solve(...)` 历史解释之上，或建立在等价的共享记录索引之上。

#### Scenario: 当前记录视图共享同一口径
- **WHEN** `record list`、浏览工作台文件视角或其他当前状态视图展示某个文件的当前记录
- **THEN** 这些能力必须基于同一条“该文件当前最新记录”输出结果
- **AND** 系统不得因入口不同而各自选择不同的当前记录

#### Scenario: 题目时间线与统计钻取共享同一历史解释
- **WHEN** 用户从浏览、统计或复习建议入口钻取某个题目或文件的历史
- **THEN** 系统必须使用与其他历史派生能力一致的 `solve(...)` 历史解释和 revision 解析结果
- **AND** 系统不得为某个入口单独定义另一套时间线语义

#### Scenario: 新增历史功能不得各自重定义记录语义
- **WHEN** 后续新增任何基于历史的浏览、建议、筛选或维护能力
- **THEN** 实现必须复用现有共享读模型或与其等价的统一索引
- **AND** 不得重新发明另一套“最新记录”“当前状态”或“时间线”规则

### Requirement: 历史派生与记录维护必须通过统一仓库访问层完成
历史派生能力和记录维护能力 MUST 通过统一的仓库访问层读取或修改本地 `jj` 历史，而不得在多个 workflow 中散落直接的 `jj-lib` / `jj` CLI 组合逻辑。

#### Scenario: 历史派生能力通过统一仓库层读取记录
- **WHEN** `record list`、`record show`、浏览工作台、`stats` 或复习建议需要读取历史记录
- **THEN** 系统必须通过统一仓库访问层完成工作区校验、历史读取、索引构建或 revision 解析
- **AND** workflow 层不得自行拼装底层 `jj` 读取路径

#### Scenario: 记录维护能力通过统一仓库层改写事实
- **WHEN** `record edit`、`record rebind` 或其他记录维护能力需要改写现有训练记录
- **THEN** 系统必须通过统一仓库访问层执行对应的仓库写操作
- **AND** workflow 层不得直接散落底层 `jj` 写命令构造逻辑

### Requirement: 命令内仓库操作必须遵守串行顺序语义
在同一次 CLI 命令中，仓库访问层 MUST 保证仓库操作遵守串行顺序语义，使后续读操作能够观察到先前写操作已经生效后的状态。

#### Scenario: 写后读观察到最新仓库状态
- **WHEN** 某次命令执行中先完成一次仓库写入，再继续读取历史或工作区状态
- **THEN** 后续读取必须基于该写入之后的仓库状态执行
- **AND** 系统不得因为缓存了过期仓库视图而返回旧结果

#### Scenario: 多个仓库请求按进入顺序生效
- **WHEN** 同一次命令中连续发出多个仓库请求
- **THEN** 仓库访问层必须保证这些请求按进入顺序依次完成
- **AND** 调用方不得承担额外的顺序同步职责来防止仓库状态竞态

### Requirement: 同一进程内同一工作区的 live actor 必须保持唯一
在同一个 `aclog` 进程内，针对同一个工作区的 live 仓库访问 MUST 复用同一个 actor，而不得重复创建多个并行的 live actor 实例。

#### Scenario: 重复获取同一工作区的 live handle 复用同一 actor
- **WHEN** 同一个 `aclog` 进程内多次请求同一个工作区的 live 仓库 handle
- **THEN** 系统必须复用同一个 live actor
- **AND** 系统不得为同一工作区并行创建多个 live actor 来处理仓库请求

#### Scenario: 重复构造依赖对象不会启动第二个 actor
- **WHEN** 该工作区的 live actor 已经存在
- **THEN** 后续请求必须返回指向同一 actor 的 handle
- **AND** 系统不得因为重复构造依赖对象而启动第二个 actor 线程

### Requirement: 记录维护必须改写 `jj` 中的记录本体
针对既有训练记录的修正或补充，系统 MUST 直接改写对应 `jj` commit 的结构化描述，而不是额外维护一份侧边状态或追加“修正记录”来模拟更新。

#### Scenario: `record edit` 改写原记录
- **WHEN** 用户对某条已有 `solve(...)` 记录执行训练字段编辑
- **THEN** 系统必须重写该记录自身的 commit description
- **AND** 系统不得为这次编辑额外创建新的训练记录 commit

#### Scenario: `record rebind` 改写被选中的历史记录
- **WHEN** 用户对某条已有 `solve(...)` 记录执行 `record rebind`
- **THEN** 系统必须重写被选中的那条历史记录
- **AND** 系统不得通过追加 correction、rebind-log 或其他旁路 commit 来表达这次修正

#### Scenario: 记录修正不得依赖侧边数据库
- **WHEN** 系统需要保存某条既有记录的训练字段变更、submission 变更或其他结构化修正
- **THEN** 系统必须把修正后的值写回目标记录本体
- **AND** 系统不得把这些修正只保存在 `.aclog/*` 或其他独立数据库中

### Requirement: `.aclog/` 只能承担配置、缓存和流程恢复职责
`.aclog/` 下的文件 MUST 只承担配置、缓存和未完成流程恢复状态职责，而不得成为训练记录事实源的替代品。

#### Scenario: `.aclog/config.toml` 与缓存文件不承担记录真值
- **WHEN** 系统读取 `.aclog/config.toml`、`.aclog/problems/*.toml`、`luogu-mappings.toml` 或 `luogu-tags.toml`
- **THEN** 系统必须把它们视为配置或缓存
- **AND** 系统不得把它们解释为“当前记录状态”“题目时间线”“统计快照”或“复习建议真值”

#### Scenario: 不允许把历史派生结果回写为真值副本
- **WHEN** 系统生成当前记录状态、统计结果、题目时间线或复习建议
- **THEN** 系统不得把这些派生结果持久化为另一份可写事实源
- **AND** 后续读取时必须继续从 `jj` 历史重新推导

### Requirement: `sync` 必须被视为当前工作副本派生的明确例外
`sync` MUST 被视为“当前工作副本派生”能力，而不是“全图历史派生”能力；它可以读取历史作为辅助上下文，但其输入主体必须是当前 working-copy 与父提交之间的变更。

#### Scenario: `sync` 从当前工作副本 diff 派生待处理项
- **WHEN** 用户执行 `aclog sync`
- **THEN** 系统必须以当前 working-copy commit 与其父提交之间的 diff 作为待处理文件来源
- **AND** 系统不得把整个本地 `jj` 图重新解释为一批新的 sync 待办项

#### Scenario: `sync-session.toml` 只保存流程恢复状态
- **WHEN** `sync` 过程中存在未完成批次，需要支持暂停和恢复
- **THEN** 系统可以在 `.aclog/sync-session.toml` 中保存批次、选择结果和恢复所需状态
- **AND** 系统必须把该文件视为流程状态缓存，而不是训练记录真值

#### Scenario: `sync` 的最终训练事实来自提交结果
- **WHEN** `sync` 最终完成并落地正式记录
- **THEN** 系统必须以最终写入本地 `jj` 历史的 commit 作为训练事实
- **AND** 即使 `sync-session.toml` 被删除，已经写入的训练记录事实也必须保持成立
