## MODIFIED Requirements

### Requirement: record bind MUST 为被跟踪的解法文件创建标准 solve 记录
系统 MUST 提供 `aclog record bind <file>`，以具体解法文件为对象，为该文件创建一条标准 `solve(...)` 记录。新记录 MUST 使用全局题目标识写入 commit 头部，并允许目标文件来自任一受支持 provider。

#### Scenario: 为 Luogu 文件补录 submission
- **WHEN** 用户执行 `aclog record bind <file>`，且该文件存在、被当前 `jj` 工作区跟踪，并且文件名可解析为受支持的 Luogu 题目标识
- **THEN** 系统必须为该文件拉取同题题目元数据和 submission 列表
- **AND** 系统必须在选定一条 submission 后创建以全局题目标识写入的标准 `solve(...)` commit

#### Scenario: 为 AtCoder 文件补录 submission
- **WHEN** 用户执行 `aclog record bind <file>`，且该文件存在、被当前 `jj` 工作区跟踪，并且文件名可解析为受支持的 AtCoder task 标识
- **THEN** 系统必须为该文件拉取对应题目的元数据和 submission 列表
- **AND** 系统必须创建以 `atcoder:<task-id>` 形式写入的标准 `solve(...)` commit

#### Scenario: bind 遇到未被支持的文件
- **WHEN** 用户执行 `aclog record bind <file>`，且该文件名不能解析为任何受支持 provider 的题目标识
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统必须明确提示该文件不属于当前受支持的题目源命名范围

### Requirement: record rebind MUST 重写用户选中的同文件 solve 记录
系统 MUST 提供 `aclog record rebind <file>`，并通过 `jj` rewrite 修正该文件既有 `solve(...)` 记录绑定到哪条 submission。重绑时系统必须保留该记录既有的训练字段，只更新 submission 相关信息和由题目 metadata 派生的字段。

#### Scenario: rebind 使用多 provider 全局题目标识
- **WHEN** 用户对一个来自受支持 provider 的文件执行 `record rebind`
- **THEN** 系统必须只在与该文件全局题目标识相同的 submission 候选中完成重绑
- **AND** 系统不得因为不同 provider 的原始题号相似而跨 provider 改绑

#### Scenario: rebind 遇到未被支持的文件
- **WHEN** 用户执行 `aclog record rebind <file>`，且该文件名不能解析为任何受支持 provider 的题目标识
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统不得进入旧记录选择或 submission 选择流程

### Requirement: record 的交互式选择器必须采用统一终端布局
系统 MUST 让 `record bind` 的 submission 选择器和 `record rebind` 的旧记录/新 submission 选择器采用统一的终端布局，明确区分题目上下文、provider 信息、比赛上下文、候选列表、详情摘要与操作提示。

#### Scenario: Luogu submission 选择器展示上下文
- **WHEN** 用户在未显式提供 `--submission-id` 的情况下对 Luogu 文件执行 `record bind`
- **THEN** 选择界面必须展示全局题目标识、题目标题、来源以及难度或标签等上下文
- **AND** submission 列表必须与当前可执行动作提示分区展示

#### Scenario: AtCoder submission 选择器展示上下文
- **WHEN** 用户在未显式提供 `--submission-id` 的情况下对 AtCoder 文件执行 `record bind` 或 `record rebind`
- **THEN** 选择界面必须展示 `Source: AtCoder`
- **AND** 若当前题目存在比赛信息，界面必须同时展示 `Contest`
- **AND** submission 列表必须继续与当前可执行动作提示分区展示
