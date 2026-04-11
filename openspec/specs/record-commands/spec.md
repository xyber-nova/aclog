## Purpose

为当前工作区提供按解法文件组织的记录管理能力，支持补录、重绑和列表查看，并保持命令语义与 TUI 交互边界清晰。

## Requirements

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

#### Scenario: bind 遇到未被跟踪的文件
- **WHEN** 用户执行 `aclog record bind <file>`，且该文件未被当前 `jj` 工作区跟踪
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统不得为该文件创建任何 commit

#### Scenario: bind 遇到非受支持题号文件
- **WHEN** 用户执行 `aclog record bind <file>`，且该文件名不能解析为任何受支持 provider 的题目标识
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统必须明确提示该文件不属于当前受支持的题目源命名范围

### Requirement: record rebind MUST 重写用户选中的同文件 solve 记录
系统 MUST 提供 `aclog record rebind <file>`，并通过 `jj` rewrite 修正该文件既有 `solve(...)` 记录绑定到哪条 submission。重绑时系统必须保留该记录既有的训练字段，只更新 submission 相关信息和由题目 metadata 派生的字段。

#### Scenario: rebind 使用多 provider 全局题目标识
- **WHEN** 用户对一个来自受支持 provider 的文件执行 `record rebind`
- **THEN** 系统必须只在与该文件全局题目标识相同的 submission 候选中完成重绑
- **AND** 系统不得因为不同 provider 的原始题号相似而跨 provider 改绑

#### Scenario: 文件没有可重写的 solve 记录
- **WHEN** 用户执行 `aclog record rebind <file>`，且该文件在本地历史中不存在任何可识别的 `solve(...)` 记录
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统必须明确提示该文件当前没有可重绑的记录

#### Scenario: rebind 遇到非受支持题号文件
- **WHEN** 用户执行 `aclog record rebind <file>`，且该文件名不能解析为任何受支持 provider 的题目标识
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统不得进入旧记录选择或 submission 选择流程

#### Scenario: rebind 不能跨题改绑
- **WHEN** 用户对某个文件执行 `record rebind` 并选择新的 submission
- **THEN** 系统必须只接受与该文件题号相同的 submission
- **AND** 系统不得把该记录改绑到另一道题

#### Scenario: rebind 保留训练字段
- **WHEN** 被重写的历史记录已经包含训练字段
- **THEN** 系统必须在重写后的记录中保留这些训练字段
- **AND** 系统不得因为更换 submission 而清空这些字段

### Requirement: record list MUST 按文件展示当前记录状态
系统 MUST 提供 `aclog record list`，按文件列出当前工作区已记录解法文件的当前状态，而不是按题目聚合。该命令必须支持过滤条件和结构化输出，但不同输出模式必须共享同一套“每文件最新记录”口径。

#### Scenario: 同一文件存在多条 solve 历史
- **WHEN** 某个文件在本地历史中存在多条可识别的 `solve(...)` 记录
- **THEN** `record list` 必须只展示该文件最新的一条记录作为当前状态

#### Scenario: 同一道题存在多个文件记录
- **WHEN** 多个不同文件都绑定到了同一个 `problem-id`
- **THEN** `record list` 必须为这些文件分别输出独立记录
- **AND** 系统不得把它们合并成一条按题目聚合的结果

#### Scenario: 使用过滤参数列出当前记录
- **WHEN** 用户执行 `record list` 并提供题号、文件名、结果、难度或标签过滤参数
- **THEN** 系统必须按这些条件过滤输出结果
- **AND** 过滤必须作用于“当前状态”而不是完整历史

#### Scenario: 使用结构化输出模式
- **WHEN** 用户执行 `record list` 并请求结构化输出
- **THEN** 系统必须输出与文本模式等价的数据集合
- **AND** 系统不得因为输出格式不同而改变记录选择口径

### Requirement: record 的选择步骤 MUST 可由非交互 CLI 完全表达
系统 MUST NOT 把 `record` 的关键选择能力做成交互界面独占行为；所有选择步骤 MUST 有等价的非交互 CLI 输入方式。

#### Scenario: bind 通过 CLI 直接指定 submission
- **WHEN** 用户执行 `aclog record bind <file> --submission-id <id>`
- **THEN** 系统必须直接使用该 submission 完成绑定
- **AND** 系统不得再要求用户进入 submission 选择 TUI

#### Scenario: bind 指定的 submission 不属于同题
- **WHEN** 用户执行 `aclog record bind <file> --submission-id <id>`，且该 submission 不属于目标文件解析出的题号
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统不得回退到 submission 选择 TUI

#### Scenario: rebind 通过 CLI 完成全部选择
- **WHEN** 用户执行 `aclog record rebind <file> --record-rev <revset> --submission-id <id>`
- **THEN** 系统必须直接重写由 `--record-rev` 指定的那条历史记录，并将其改绑到指定 submission
- **AND** 系统不得再要求用户进入任何 TUI 选择步骤

#### Scenario: rebind 指定的旧记录不匹配目标文件
- **WHEN** 用户执行 `aclog record rebind <file> --record-rev <revset>`，且该 revset 没有唯一解析到一条匹配目标文件的标准 `solve(...)` 记录
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统不得回退到旧记录选择 TUI

#### Scenario: CLI 只补齐部分选择
- **WHEN** 用户执行 `record rebind` 或 `record bind`，并且 CLI 参数只补齐了一部分选择
- **THEN** 系统必须只为剩余未决选择进入 TUI
- **AND** 已由 CLI 明确指定的选择不得要求用户再次确认

#### Scenario: record list 始终使用 CLI 输出
- **WHEN** 用户执行 `aclog record list`
- **THEN** 系统必须直接以 CLI 文本形式输出记录结果
- **AND** 系统不得进入 TUI 界面

### Requirement: record MUST 提供记录详情查看命令
系统 MUST 提供针对具体解法文件的记录详情查看命令，用于展示最新记录或指定历史记录的完整字段详情。

#### Scenario: 查看文件当前记录详情
- **WHEN** 用户执行记录详情查看命令且未指定 `--record-rev`
- **THEN** 系统必须展示该文件最新一条标准 `solve(...)` 记录的完整详情

#### Scenario: 查看指定 revision 的记录详情
- **WHEN** 用户执行记录详情查看命令并通过 `--record-rev` 指定一条唯一历史记录
- **THEN** 系统必须展示该记录的完整字段详情
- **AND** 如果该 revision 不匹配目标文件，系统必须拒绝执行

#### Scenario: 查看详情时遇到非受支持题号文件
- **WHEN** 用户对一个文件名不属于当前支持洛谷题号范围的文件执行记录详情查看命令
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统不得尝试猜测题号或继续读取记录

### Requirement: record MUST 提供训练字段编辑命令
系统 MUST 提供针对具体解法文件的训练字段编辑命令，并允许通过 CLI 参数完整表达目标记录和字段变更。

#### Scenario: 通过 CLI 编辑训练字段
- **WHEN** 用户执行训练字段编辑命令并通过参数提供一个或多个训练字段值
- **THEN** 系统必须直接重写目标记录中的对应字段
- **AND** 系统不得强制要求进入 TUI

#### Scenario: 只更新部分训练字段
- **WHEN** 用户执行训练字段编辑命令且只提供部分训练字段参数
- **THEN** 系统必须只更新这些已提供字段
- **AND** 其他未提供字段必须保持原值

#### Scenario: 编辑训练字段时遇到非受支持题号文件
- **WHEN** 用户对一个文件名不属于当前支持洛谷题号范围的文件执行训练字段编辑命令
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统不得尝试定位或重写任何记录

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

### Requirement: record 的交互式选择器必须遵循统一工作流键位
系统 MUST 让 `record bind` 与 `record rebind` 的交互式选择器遵循统一工作流键位：`Enter` 确认当前焦点选择，`Esc` 返回上一步或取消当前选择器，`q` 允许直接退出当前工作台，`j/k` 与方向键等价。

#### Scenario: submission 选择器取消
- **WHEN** 用户在 `record bind` 或 `record rebind` 的 submission 选择器中按下 `Esc`
- **THEN** 系统必须取消当前选择器并返回上一步或命令调用方
- **AND** 系统不得将该操作解释为其他业务动作

### Requirement: record 的交互式选择器必须展示当前选中项摘要
系统 MUST 在 record 相关选择器中为当前选中项提供摘要或详情区域，使用户在确认前能看到关键 submission 或历史记录信息，而不必只通过表格列猜测。

#### Scenario: submission 选择器移动焦点
- **WHEN** 用户在 `record bind` 或 `record rebind` 的 submission 列表中移动选中项
- **THEN** 页面必须同步展示当前 submission 的结果、分数、时间、内存和提交时间等关键摘要

#### Scenario: 旧记录选择器移动焦点
- **WHEN** 用户在 `record rebind` 的旧记录列表中移动选中项
- **THEN** 页面必须同步展示该记录的 revision、结果和提交时间等关键摘要
