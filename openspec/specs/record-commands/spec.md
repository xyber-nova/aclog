## Purpose

为当前工作区提供按解法文件组织的记录管理能力，支持补录、重绑和列表查看，并保持命令语义与 TUI 交互边界清晰。

## ADDED Requirements

### Requirement: record bind MUST 为被跟踪的解法文件创建标准 solve 记录
系统 MUST 提供 `aclog record bind <file>`，以具体解法文件为对象，为该文件创建一条标准 `solve(...)` 记录。

#### Scenario: 为被跟踪文件补录 submission
- **WHEN** 用户执行 `aclog record bind <file>`，且该文件存在、被当前 `jj` 工作区跟踪、并且文件名可提取题号
- **THEN** 系统必须为该文件拉取同题题目元数据和 submission 列表
- **AND** 系统必须在选定一条 submission 后创建标准 `solve(...)` commit

#### Scenario: bind 遇到未被跟踪的文件
- **WHEN** 用户执行 `aclog record bind <file>`，且该文件未被当前 `jj` 工作区跟踪
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统不得为该文件创建任何 commit

### Requirement: record rebind MUST 重写用户选中的同文件 solve 记录
系统 MUST 提供 `aclog record rebind <file>`，并通过 `jj` rewrite 修正该文件既有 `solve(...)` 记录绑定到哪条 submission。

#### Scenario: 同一文件存在多条 solve 记录
- **WHEN** 用户执行 `aclog record rebind <file>`，且该文件在本地历史中存在多条可识别的 `solve(...)` 记录
- **THEN** 系统必须先让用户在这些历史记录中选择要重写的那一条
- **AND** 系统必须再让用户从同题 submission 中选择新的绑定结果
- **AND** 系统必须重写被选中的那条 `solve(...)` 记录，而不是追加新的 correction commit

#### Scenario: 文件没有可重写的 solve 记录
- **WHEN** 用户执行 `aclog record rebind <file>`，且该文件在本地历史中不存在任何可识别的 `solve(...)` 记录
- **THEN** 系统必须拒绝执行该命令
- **AND** 系统必须明确提示该文件当前没有可重绑的记录

#### Scenario: rebind 不能跨题改绑
- **WHEN** 用户对某个文件执行 `record rebind` 并选择新的 submission
- **THEN** 系统必须只接受与该文件题号相同的 submission
- **AND** 系统不得把该记录改绑到另一道题

### Requirement: record list MUST 按文件展示当前记录状态
系统 MUST 提供 `aclog record list`，按文件列出当前工作区已记录解法文件的当前状态，而不是按题目聚合。

#### Scenario: 同一文件存在多条 solve 历史
- **WHEN** 某个文件在本地历史中存在多条可识别的 `solve(...)` 记录
- **THEN** `record list` 必须只展示该文件最新的一条记录作为当前状态

#### Scenario: 同一道题存在多个文件记录
- **WHEN** 多个不同文件都绑定到了同一个 `problem-id`
- **THEN** `record list` 必须为这些文件分别输出独立记录
- **AND** 系统不得把它们合并成一条按题目聚合的结果

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
