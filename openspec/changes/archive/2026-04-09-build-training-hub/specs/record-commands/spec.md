## MODIFIED Requirements

### Requirement: record rebind MUST 重写用户选中的同文件 solve 记录
系统 MUST 提供 `aclog record rebind <file>`，并通过 `jj` rewrite 修正该文件既有 `solve(...)` 记录绑定到哪条 submission。重绑时系统必须保留该记录既有的训练字段，只更新 submission 相关信息和由题目 metadata 派生的字段。

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

## ADDED Requirements

### Requirement: record MUST 提供记录详情查看命令
系统 MUST 提供针对具体解法文件的记录详情查看命令，用于展示最新记录或指定历史记录的完整字段详情。

#### Scenario: 查看文件当前记录详情
- **WHEN** 用户执行记录详情查看命令且未指定 `--record-rev`
- **THEN** 系统必须展示该文件最新一条标准 `solve(...)` 记录的完整详情

#### Scenario: 查看指定 revision 的记录详情
- **WHEN** 用户执行记录详情查看命令并通过 `--record-rev` 指定一条唯一历史记录
- **THEN** 系统必须展示该记录的完整字段详情
- **AND** 如果该 revision 不匹配目标文件，系统必须拒绝执行

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
