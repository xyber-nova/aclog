## Purpose

为当前工作区提供基于本地 `jj` 做题历史的统计能力，并在需要时借助洛谷标签类型字典生成人工筛过的算法标签统计。

## Requirements

### Requirement: stats 命令必须提供独立的本地统计入口
系统 MUST 提供 `aclog stats` 命令，用于在指定工作区中展示基于本地历史的统计与建议视图，而不改变现有 `sync` 流程。该命令必须支持时间窗口或视图模式参数，以便在同一入口下切换概览统计和复习建议。

#### Scenario: 使用默认工作区运行 stats
- **WHEN** 用户执行 `aclog stats`
- **THEN** 系统必须以当前目录作为工作区读取本地统计数据
- **AND** 系统必须进入统计界面而不是执行 `sync`

#### Scenario: 使用显式工作区和时间窗口运行 stats
- **WHEN** 用户执行 `aclog stats --workspace /path/to/workspace` 并指定时间窗口参数
- **THEN** 系统必须从该路径加载工作区并按该时间窗口生成统计结果

### Requirement: stats 命令必须基于本地 solve 历史生成统计
系统 MUST 以当前工作区本地 `jj` 历史中的 `solve(...)` commit 作为做题统计的主体数据源；`chore(...)`、`remove(...)` 以及无法识别的提交不得计入统计。为完成算法标签分类，系统可以读取本地或远端的标签类型字典。

#### Scenario: 工作区包含 solve、chore 和 remove 提交
- **WHEN** 统计流程遍历工作区本地历史
- **THEN** 系统必须只把 `solve(...)` commit 计入做题统计
- **AND** 系统不得把 `chore(...)` 或 `remove(...)` commit 计入结果

#### Scenario: 历史中存在非项目格式提交
- **WHEN** 统计流程遇到不匹配项目 `solve(...)` message 结构的提交
- **THEN** 系统必须忽略这些提交
- **AND** 系统不得为其猜测题号、结果或难度

#### Scenario: 标签类型字典缓存缺失
- **WHEN** 用户执行 `aclog stats` 且本地标签类型字典缓存不存在或已过期
- **THEN** 系统可以请求 `/_lfe/tags` 刷新标签字典
- **AND** 系统必须继续以本地 `solve(...)` 历史作为做题统计主体数据源

### Requirement: stats 的标签统计必须只使用算法标签
系统 MUST 仅使用算法标签生成标签统计；来源、时间、地区、特殊题目等非算法标签不得进入统计结果，但 `ProblemMetadata.tags` 和 `solve(...)` commit 中的原始标签不得被截断。

#### Scenario: 题目标签同时包含算法与非算法标签
- **WHEN** 系统从 `/_lfe/tags` 和题目元数据解析标签
- **THEN** 系统必须保留题目的原始标签信息用于元数据和 commit message
- **AND** 系统必须在统计阶段根据标签类型只保留算法标签
- **AND** 系统不得把来源、时间、地区或特殊题目标签计入统计数据

#### Scenario: 标签缓存包含类型信息
- **WHEN** 系统缓存 `/_lfe/tags` 返回的标签字典
- **THEN** 缓存结构必须保留标签名称、类型和父标签信息
- **AND** 系统必须将该缓存持久化到工作区 `.aclog` 目录中

### Requirement: stats 界面必须同时展示唯一题目和全部记录两种口径
系统 MUST 同时展示按题号去重后的唯一题目统计，以及全部 `solve(...)` 记录统计，以区分“做过多少题”和“做题活动量”。在此基础上，系统还必须能识别首次 AC、重复练习和当前非 AC 状态。

#### Scenario: 同一道题存在多条 solve 记录
- **WHEN** 某个题号在本地历史中出现多条 `solve(...)` commit
- **THEN** 系统必须在“全部记录”口径中统计全部这些记录
- **AND** 系统必须在“唯一题目”口径中只将该题统计一次

#### Scenario: 唯一题目统计需要当前状态
- **WHEN** 系统为某个题号计算唯一题目统计
- **THEN** 系统必须使用该题最新的一条 `solve(...)` 记录作为当前状态

#### Scenario: 区分首次 AC 与重复练习
- **WHEN** 某道题在历史中经历了首次 AC 后又产生后续练习记录
- **THEN** 系统必须能够区分“首次 AC 已完成”和“后续重复练习活动”
- **AND** 系统不得把这两者混为同一种指标

### Requirement: stats 界面必须展示首版概览指标
统计界面 MUST 至少展示唯一题目数、`solve` 记录数、AC 与非 AC 情况、按结果分布、按难度分布以及按算法标签分布；扩展后的统计界面还必须支持时间窗口概览和可钻取的训练建议入口。

#### Scenario: 存在本地 solve 记录
- **WHEN** 工作区中至少存在一条可识别的 `solve(...)` 记录
- **THEN** 统计界面必须展示唯一题目数与 `solve` 记录数
- **AND** 统计界面必须展示 verdict 分布摘要
- **AND** 统计界面必须展示 difficulty 分布摘要
- **AND** 统计界面必须展示 tag 分布摘要

#### Scenario: 统计字段缺失
- **WHEN** 某条 `solve(...)` 记录缺少 verdict、difficulty 或其他统计字段
- **THEN** 系统必须以占位值显示该字段
- **AND** 系统不得猜测原始值

#### Scenario: 从统计界面进入建议或钻取视图
- **WHEN** 用户在统计界面中选择某个指标、标签或建议入口
- **THEN** 系统必须能够进入对应的明细或建议视图

### Requirement: stats 界面必须提供明确空状态
当当前工作区没有任何可识别的 `solve(...)` 提交时，统计界面 MUST 展示空状态，并允许用户直接退出。

#### Scenario: 工作区没有 solve 历史
- **WHEN** 用户进入 `aclog stats` 且当前工作区没有任何可识别的 `solve(...)` commit
- **THEN** 系统必须明确提示当前工作区还没有已记录的做题提交
- **AND** 系统必须提供退出方式

### Requirement: stats 必须支持时间窗口过滤
系统 MUST 允许用户按时间窗口查看训练统计，并确保时间窗口过滤同时作用于聚合统计和建议生成。

#### Scenario: 查看最近时间窗口统计
- **WHEN** 用户请求最近 N 天或其他受支持时间窗口的统计
- **THEN** 系统必须只使用落在该时间窗口内的记录生成窗口统计结果

#### Scenario: 时间窗口外历史仍可用于当前状态判断
- **WHEN** 系统在某个时间窗口内统计唯一题目当前状态
- **THEN** 系统可以继续使用窗口外历史辅助判断首次 AC 或长期复习状态
- **AND** 系统必须明确区分“窗口内活动”与“全历史状态”

### Requirement: stats 必须暴露复习建议入口
系统 SHALL 允许用户从 `stats` 命令进入复习候选或训练建议视图，而不要求切换到另一套命令语义。

#### Scenario: stats 打开复习候选视图
- **WHEN** 用户在 `stats` 命令中请求建议模式
- **THEN** 系统必须展示基于当前工作区历史生成的复习候选或薄弱点建议
- **AND** 这些结果必须与同一工作区下的浏览工作台兼容

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
