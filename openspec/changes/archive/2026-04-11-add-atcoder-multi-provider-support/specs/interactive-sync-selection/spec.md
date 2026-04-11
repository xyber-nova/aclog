## MODIFIED Requirements

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
