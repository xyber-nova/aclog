## MODIFIED Requirements

### Requirement: sync 必须为每个变更题目文件要求显式选择
当 `aclog sync` 检测到题目文件发生变更时，系统 MUST 先建立当前批次的待处理清单，并在生成 commit 之前为每个文件保留显式选择步骤。系统可以提供默认候选或批量预览，但默认候选不得等价于已确认选择。

#### Scenario: 批量预览后进入单文件选择
- **WHEN** `sync` 检测到多个题目文件变更
- **THEN** 系统必须先向用户展示当前批次的预览清单
- **AND** 用户必须能够从预览清单进入任一文件的显式选择步骤

#### Scenario: 变更文件存在默认候选 submission
- **WHEN** 某个题目文件存在可用提交记录，且系统能够推断出默认候选
- **THEN** 系统可以在预览或详情界面中标记该默认候选
- **AND** 系统仍然必须等待用户显式确认后才能为该文件创建 commit

#### Scenario: 变更文件不存在提交记录
- **WHEN** `sync` 检测到某个题目文件发生变更，且没有可用的提交记录
- **THEN** 系统仍然必须为该文件展示显式选择步骤
- **AND** 系统不得因为没有记录就自动创建 `chore(...)` commit

### Requirement: sync 必须将 chore 视为用户的显式决策
系统 SHALL 只在用户于 `sync` 预览页或详情页中明确选择 `chore` 操作时创建 `chore(...)` commit；推荐候选、空状态或恢复会话都不得隐式导出该结果。

#### Scenario: 预览页中将文件标记为 chore
- **WHEN** 用户在 `sync` 的批量预览或详情界面中对某个文件显式选择 `chore`
- **THEN** 系统必须为该文件创建 `chore(problem-id): 本地修改` commit

#### Scenario: 恢复未完成批次时保留 chore 决策
- **WHEN** 用户恢复一个未完成的 `sync` 批次，且其中某个文件此前已被显式标记为 `chore`
- **THEN** 系统必须恢复该已决状态
- **AND** 系统不得将未决文件自动视为 `chore`

### Requirement: sync 必须保持 submission、chore 和 skip 为不同结果
sync 流程 MUST 将提交记录绑定、记为 `chore` 与跳过处理视为三种彼此独立的结果；批量预览中的推荐状态、告警状态或恢复状态不得改变这三种结果的语义。

#### Scenario: 用户确认推荐 submission
- **WHEN** 用户在详情页或预览页接受某个推荐 submission
- **THEN** 系统必须使用该 submission 创建标准 `solve(...)` commit

#### Scenario: 用户在预览页跳过当前文件
- **WHEN** 用户在 `sync` 的批量预览或详情界面中对某个文件执行 `skip`
- **THEN** 系统不得为该文件生成 commit
- **AND** 系统必须将该文件记录为已决但跳过的结果

### Requirement: sync 界面必须提供明确的无记录空状态
当某个变更题目文件没有可用提交记录时，sync 的预览和详情界面 MUST 明确展示无记录空状态，并继续提供可执行操作。

#### Scenario: 预览页展示无记录状态
- **WHEN** 当前批次中存在没有可用提交记录的变更文件
- **THEN** 预览页必须明确标记这些文件未找到 submission

#### Scenario: 详情页展示可执行操作
- **WHEN** 某个没有提交记录的文件打开了详情界面
- **THEN** 界面必须明确提示未找到提交记录
- **AND** 界面必须向用户展示 `chore` 与 `skip` 两个可执行操作

## ADDED Requirements

### Requirement: sync 必须支持 non-mutating 的 dry-run 预览
系统 MUST 提供 `sync --dry-run`，用于在不创建 commit 的前提下输出当前批次的待处理项、默认候选和告警信息。

#### Scenario: dry-run 预览批次
- **WHEN** 用户执行 `aclog sync --dry-run`
- **THEN** 系统必须输出当前批次中每个待处理文件的预览结果
- **AND** 系统不得创建任何 commit
- **AND** 系统不得留下可恢复的未完成批次状态

### Requirement: sync 必须支持恢复未完成批次
当 `sync` 批次在用户中断或命令异常前尚未完成时，系统 MUST 能够在后续运行中恢复该批次。

#### Scenario: 检测到未完成批次
- **WHEN** 用户重新执行 `aclog sync`，且工作区存在未完成的 `sync` 批次状态
- **THEN** 系统必须提示用户继续恢复或丢弃该批次后重建

#### Scenario: 恢复前校验批次有效性
- **WHEN** 系统尝试恢复某个未完成批次
- **THEN** 系统必须重新校验批次中的文件是否仍与当前工作区状态兼容
- **AND** 对已失效的项必须显式标记，而不是静默继续执行旧决策

### Requirement: sync 必须在提交前展示一致性告警
当系统检测到潜在误绑风险时，系统 MUST 在用户确认前展示一致性告警，并要求用户显式处理。

#### Scenario: submission 题号与文件题号不一致
- **WHEN** 用户选择的 submission 与目标文件解析出的题号不一致
- **THEN** 系统必须阻止该记录进入提交计划
- **AND** 系统不得静默创建 commit

#### Scenario: 文件已有相同 submission 绑定
- **WHEN** 某个文件的最新记录已经绑定到同一个 submission，且用户再次选择该 submission
- **THEN** 系统必须向用户展示重复绑定提示
- **AND** 系统必须要求用户显式确认是否继续
