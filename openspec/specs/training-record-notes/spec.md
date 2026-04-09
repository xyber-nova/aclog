## Purpose

为标准 `solve(...)` 记录增加用户自己的训练上下文，并保证这些信息可以被查看、编辑、重绑和统计流程稳定复用。

## Requirements

### Requirement: solve 记录必须支持可选训练字段
系统 MUST 允许标准 `solve(...)` 记录在保留现有提交字段的同时，附带可选训练字段，包括 `Note`、`Mistakes`、`Insight`、`Confidence`、`Source-Kind` 和 `Time-Spent`。

#### Scenario: 创建带训练字段的新记录
- **WHEN** 用户通过 `record bind`、`sync` 后续编辑或其他正式入口为某条 `solve(...)` 记录提供训练字段
- **THEN** 系统必须将这些训练字段以标准化字段名写入该记录
- **AND** 该记录仍然必须保持标准 `solve(problem-id): ...` commit message 结构

#### Scenario: 旧记录缺失训练字段
- **WHEN** 系统解析一条不包含任何训练字段的历史 `solve(...)` 记录
- **THEN** 系统必须继续将其识别为有效记录
- **AND** 系统必须把缺失字段视为空值，而不是拒绝解析或猜测内容

### Requirement: 训练字段必须可在不修改题解文件内容的前提下编辑
系统 MUST 提供针对既有 `solve(...)` 记录的训练字段编辑能力；该操作只能重写目标记录描述，不得修改对应题解文件内容。

#### Scenario: 编辑指定 revision 的训练字段
- **WHEN** 用户对某个文件执行训练字段编辑，并通过 `--record-rev` 指定了一条唯一历史记录
- **THEN** 系统必须只重写该条记录中的训练字段
- **AND** 系统不得修改题目文件内容或额外创建新的训练记录

#### Scenario: 编辑当前文件最新记录
- **WHEN** 用户对某个文件执行训练字段编辑且未指定 `--record-rev`
- **THEN** 系统必须将该文件最新的一条标准 `solve(...)` 记录视为编辑目标
- **AND** 如果该文件没有任何可识别的 `solve(...)` 记录，系统必须拒绝执行

### Requirement: rebind 必须保留既有训练字段
当用户对某条已有记录执行 `record rebind` 时，系统 MUST 保留该记录原有的训练字段，仅更新 submission 绑定及由题目 metadata 派生的字段。

#### Scenario: 重绑已有训练备注的记录
- **WHEN** 某条 `solve(...)` 记录已经包含 `Note`、`Mistakes` 或其他训练字段，且用户对其执行 `record rebind`
- **THEN** 系统必须在重写后的记录中保留这些训练字段
- **AND** 系统不得因为 submission 变更而清空训练字段

#### Scenario: 重绑没有训练字段的旧记录
- **WHEN** 用户对一条旧格式 `solve(...)` 记录执行 `record rebind`
- **THEN** 系统必须允许该操作完成
- **AND** 重写后的记录可以继续缺失训练字段，直到用户显式补充
