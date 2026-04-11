## Purpose

为 `sync` 提供显式、可恢复、可预览并带防错提示的交互式记录流程，确保工作区变更在生成 commit 前都经过用户确认。

## Requirements

### Requirement: sync 只应为受支持的题目源文件建立批次项
当 `aclog sync` 从当前工作副本收集变更时，系统 MUST 只把能够解析为当前受支持 provider 题目标识的文件纳入待处理批次；既不属于受支持题目源、也无法解析为有效 provider-specific 题目标识的文件不得进入预览、详情或恢复后的批次列表。

#### Scenario: 工作区同时存在 Luogu、AtCoder 与非题目文件
- **WHEN** `sync` 检测到 Luogu 题号文件、AtCoder task 文件和其他命名风格文件同时发生变更
- **THEN** 系统必须只为可识别的受支持题目源文件创建批次项
- **AND** 系统不得把其他文件显示为待处理项或失效项

#### Scenario: 恢复旧批次时清理不再受支持的项
- **WHEN** 用户恢复一个旧的 `sync` 批次，其中包含当前规则下不再受支持的题号项
- **THEN** 系统必须在合并恢复状态时丢弃这些项
- **AND** 系统不得继续把它们展示为 `Invalid`

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

### Requirement: sync 预览页必须同时展示批次列表与当前项摘要
系统 MUST 在 sync 预览页中同时展示批次列表和当前选中项的摘要信息，使用户在进入详情前就能理解该文件的全局题目标识、来源、比赛上下文、状态、默认候选和告警情况。

#### Scenario: 预览页浏览 Luogu 项
- **WHEN** 用户在 sync 预览页中移动到某个 Luogu 项
- **THEN** 界面必须同步展示该项的文件、全局题目标识、来源、当前状态、submission 数量、默认候选和告警摘要
- **AND** Luogu 项可以不显示比赛信息

#### Scenario: 预览页浏览 AtCoder 项
- **WHEN** 用户在 sync 预览页中移动到某个 AtCoder 项
- **THEN** 列表必须展示来源列
- **AND** 摘要区域必须展示 `Source: AtCoder`
- **AND** 若当前项存在比赛上下文，摘要区域必须展示该比赛信息

### Requirement: sync 预览页必须支持安全的快速决策
系统 MUST 允许用户在 sync 预览页中直接完成不依赖 submission 列表细节的安全决策，以减少重复进入详情页的操作成本；submission 绑定仍 MUST 通过详情页完成。

#### Scenario: active 项在预览页直接标记 chore 或 skip
- **WHEN** 用户在 sync 预览页中聚焦一个 active 项并执行快速决策
- **THEN** 系统必须允许将该项直接标记为 `chore` 或 `skip`
- **AND** 系统不得要求用户必须先进入详情页

#### Scenario: deleted 项在预览页直接确认 remove
- **WHEN** 用户在 sync 预览页中聚焦一个 deleted 项并执行快速删除决策
- **THEN** 系统必须允许将该项直接确认为 `remove`
- **AND** 系统不得对 active 项错误暴露相同的删除快捷动作

#### Scenario: submission 绑定仍通过详情页完成
- **WHEN** 用户希望为某个 active 项选择具体 submission
- **THEN** 系统必须要求用户进入详情页完成该选择
- **AND** 系统不得在预览页中引入等价于完整 submission selector 的复杂交互

### Requirement: sync 详情页必须显式区分提交列表与上下文摘要
系统 MUST 在 sync 单项详情页中将 submission 列表与题目/文件上下文、provider 信息、比赛上下文、告警信息和当前可执行动作分区展示，而不是把这些信息混在单一文本块中。

#### Scenario: 详情页展示 Luogu 上下文
- **WHEN** 用户打开某个 Luogu active 变更文件的 sync 详情页
- **THEN** 页面必须展示全局题目标识、文件路径、来源、题目标题、默认候选、告警信息和当前可执行动作
- **AND** submission 列表必须与这些摘要信息处于独立区域

#### Scenario: 详情页展示 AtCoder 上下文
- **WHEN** 用户打开某个 AtCoder active 变更文件的 sync 详情页
- **THEN** 页面必须展示 `Source: AtCoder`
- **AND** 若该题存在比赛信息，页面必须展示对应 `Contest`
- **AND** submission 列表必须继续与这些上下文摘要分区显示

#### Scenario: 详情页展示无记录空状态
- **WHEN** 用户打开一个没有可用 submission 的 sync 详情页
- **THEN** 页面必须明确展示无记录空状态
- **AND** 页面必须继续显式展示 `chore` 与 `skip` 等可执行动作

### Requirement: sync 必须遵循统一的返回与退出语义
系统 MUST 让 sync 交互遵循统一的工作流语义：`Enter` 用于进入详情或确认当前动作，`Esc` 用于返回上一层，`q` 用于退出当前工作台；`skip` 必须使用显式动作键，而不得继续与 `Esc` 复用。

#### Scenario: 从详情页返回预览页
- **WHEN** 用户位于 sync 单项详情页并按下 `Esc`
- **THEN** 系统必须返回批次预览页
- **AND** 系统不得将该操作解释为 `skip`

#### Scenario: 在预览页退出 sync 工作台
- **WHEN** 用户位于 sync 预览页并按下 `q`
- **THEN** 系统必须退出当前 sync 工作台
- **AND** 如果系统需要保留未完成批次，必须继续保留已有的恢复语义

### Requirement: sync 交互页面必须显式展示批次状态语义
系统 MUST 在 sync 预览与详情界面中显式区分待处理、已决待提交、已跳过、已提交和已失效等批次状态，并为告警和失效项提供可识别的视觉提示。

#### Scenario: 预览页存在已失效或告警项
- **WHEN** sync 批次中存在已失效项或带一致性告警的项
- **THEN** 预览页必须为这些项展示显式状态与告警提示
- **AND** 用户必须能够在摘要区域理解这些状态的含义

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

### Requirement: sync 可以并发预取提交记录但不得改变交互语义
当当前批次包含多个受支持的 active 题目文件时，系统 MAY 以有界并发方式预取各题的 submission 列表，以缩短进入预览页前的等待时间；无论是否并发，预览顺序、错误语义和显式选择步骤 MUST 保持不变。

#### Scenario: 批次中存在多个 active 题目文件
- **WHEN** `sync` 为多个不同题号的 active 项准备预览数据
- **THEN** 系统可以并发拉取这些题号对应的 submission 列表
- **AND** 预览页中的文件顺序必须继续与工作区批次顺序一致

#### Scenario: 详情页与提交阶段复用已预取的 submission
- **WHEN** 某个题号的 submission 列表已经在当前 `sync` 运行中被成功预取
- **THEN** 系统应复用该结果为详情页和最终 commit 构建服务
- **AND** `.aclog/sync-session.toml` 仍只承担恢复状态职责，不应把完整 submission 列表写成新的事实源
