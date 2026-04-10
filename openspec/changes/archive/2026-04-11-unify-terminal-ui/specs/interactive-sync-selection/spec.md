## ADDED Requirements

### Requirement: sync 预览页必须同时展示批次列表与当前项摘要
系统 MUST 在 sync 预览页中同时展示批次列表和当前选中项的摘要信息，使用户在进入详情前就能理解该文件的题号、状态、默认候选和告警情况。

#### Scenario: 预览页浏览待处理项
- **WHEN** 用户在 sync 预览页中移动当前选中项
- **THEN** 界面必须同步展示该项的文件、题号、变更类型、当前状态、submission 数量、默认候选和告警摘要
- **AND** 用户不得必须进入详情页后才能知道该项是否存在风险或推荐候选

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
系统 MUST 在 sync 单项详情页中将 submission 列表与题目/文件上下文、告警信息和当前可执行动作分区展示，而不是把这些信息混在单一文本块中。

#### Scenario: 详情页展示可确认上下文
- **WHEN** 用户打开某个 active 变更文件的 sync 详情页
- **THEN** 页面必须展示题号、文件路径、题目标题、默认候选、告警信息和当前可执行动作
- **AND** submission 列表必须与这些摘要信息处于独立区域

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
