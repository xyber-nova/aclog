## ADDED Requirements

### Requirement: record 的交互式选择器必须采用统一终端布局
系统 MUST 让 `record bind` 的 submission 选择器和 `record rebind` 的旧记录/新 submission 选择器采用统一的终端布局，明确区分问题上下文、候选列表、详情摘要与操作提示。

#### Scenario: bind 选择 submission
- **WHEN** 用户在未显式提供 `--submission-id` 的情况下执行 `record bind`
- **THEN** 选择界面必须展示题号、题目标题、难度或标签等上下文
- **AND** submission 列表必须与当前可执行动作提示分区展示

#### Scenario: rebind 选择旧记录
- **WHEN** 用户在未显式提供 `--record-rev` 的情况下执行 `record rebind`
- **THEN** 旧记录选择界面必须展示文件、题号和候选记录列表
- **AND** 页面必须显式提示当前支持的确认与取消动作

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
