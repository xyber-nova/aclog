## ADDED Requirements

### Requirement: sync 必须为每个变更题目文件要求显式选择
当 `aclog sync` 检测到题目文件发生变更时，系统必须在生成 commit 之前，为该文件展示交互式选择步骤。

#### Scenario: 变更文件存在提交记录
- **WHEN** `sync` 检测到某个题目文件发生变更，且存在可用的提交记录
- **THEN** 系统必须先展示该文件的提交记录选择界面，然后才能创建 commit

#### Scenario: 变更文件不存在提交记录
- **WHEN** `sync` 检测到某个题目文件发生变更，且没有可用的提交记录
- **THEN** 系统仍然必须为该文件展示交互式选择界面，而不是自动创建 commit

### Requirement: sync 必须将 chore 视为用户的显式决策
系统只有在用户于 sync 界面中明确选择 `chore` 操作时，才可以创建 `chore(...)` commit。

#### Scenario: 有提交记录时用户选择 chore
- **WHEN** 界面展示了提交记录，且用户触发了 `chore` 操作
- **THEN** 系统必须为该文件创建 `chore(problem-id): 本地修改` commit

#### Scenario: 无提交记录时用户选择 chore
- **WHEN** 没有可用的提交记录，且用户触发了 `chore` 操作
- **THEN** 系统必须为该文件创建 `chore(problem-id): 本地修改` commit

### Requirement: sync 必须保持 submission、chore 和 skip 为不同结果
sync 流程必须将提交记录绑定、记为 `chore`、以及跳过处理视为三种彼此独立的结果。

#### Scenario: 用户确认某条提交记录
- **WHEN** 用户在 sync 界面中选择了一条具体的提交记录
- **THEN** 系统必须使用该提交记录的元数据创建 `solve(...)` commit

#### Scenario: 用户跳过当前文件
- **WHEN** 用户在某个变更文件的 sync 界面中执行了 `skip` 操作
- **THEN** 系统不得为该文件生成 commit

### Requirement: sync 界面必须提供明确的无记录空状态
当某个变更题目文件没有可用提交记录时，sync 界面必须展示说明“未找到提交记录”的空状态，并继续提供可执行操作。

#### Scenario: 空状态展示可执行操作
- **WHEN** 某个没有提交记录的文件打开了 sync 界面
- **THEN** 界面必须明确提示未找到提交记录
- **AND** 界面必须向用户展示 `chore` 与 `skip` 两个可执行操作
