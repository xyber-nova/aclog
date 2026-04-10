## ADDED Requirements

### Requirement: stats 界面必须以分区概览方式展示核心指标
系统 MUST 在 stats 界面中以清晰分区的概览区域展示核心训练指标，使用户能够快速区分总体数量、当前状态和分布信息，而不是从单一文本段落中自行解析。

#### Scenario: 打开默认 stats 概览
- **WHEN** 用户执行 `aclog stats` 并进入默认统计界面
- **THEN** 页面必须显式展示工作区上下文与统计范围
- **AND** 页面必须将总体指标与结果/难度/标签分布分区展示

### Requirement: stats 与 review 视图必须共享主题但保持模式可辨识
系统 MUST 让 stats 概览模式与 review 建议模式共享统一终端主题，同时清晰区分“统计概览”和“建议工作台”两种模式，避免用户混淆当前界面的操作目标。

#### Scenario: 从概览切换到 review
- **WHEN** 用户在 stats 界面中进入 review 候选视图
- **THEN** 页面必须明确显示当前处于建议模式
- **AND** 建议列表、建议详情和可钻取动作必须与统计概览形成清晰区分

### Requirement: stats 工作台必须以页签方式切换 overview 与 review
系统 MUST 将 stats 概览和 review 建议实现为同一工作台内的同层模式；`Tab` MUST 作为主模式切换键，`Esc` MUST 在 review 中先返回 overview，并在 overview 中作为退出当前工作台的允许方式，`q` MUST 在任意模式下直接退出。

#### Scenario: 在 overview 与 review 之间切换
- **WHEN** 用户位于 stats 工作台并按下 `Tab`
- **THEN** 系统必须在 overview 与 review 之间切换
- **AND** 如果目标模式当前没有数据，系统必须显示该模式的空状态而不是拒绝切换

#### Scenario: 从 review 返回 overview
- **WHEN** 用户位于 review 模式并按下 `Esc`
- **THEN** 系统必须返回 overview 模式
- **AND** 系统不得直接退出整个 stats 工作台

### Requirement: stats 页面必须显式展示帮助与钻取入口
系统 MUST 在 stats 页面中显式展示当前支持的钻取入口和帮助提示，使用户知道如何进入 review、文件浏览、题目浏览或退出当前页面。

#### Scenario: stats 页面显示当前操作
- **WHEN** 用户停留在 stats 概览或 review 页面
- **THEN** 页面必须显示当前模式下可用的核心动作
- **AND** 帮助显示不得改变当前统计结果或建议结果

#### Scenario: stats 保留跨工作台快捷跳转
- **WHEN** 用户在 stats 工作台中触发文件浏览或题目浏览快捷入口
- **THEN** 系统必须继续支持直接跳转到对应的浏览工作台
- **AND** 这些快捷键必须与 `Tab` 的同层模式切换语义区分开
