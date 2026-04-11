## Purpose

为当前工作区提供基于本地 `jj` 做题历史的统计能力，并在需要时借助算法标签字典生成人工筛过的标签统计。

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
系统 MUST 以当前工作区本地 `jj` 历史中的 `solve(...)` commit 作为做题统计的主体数据源；`chore(...)`、`remove(...)` 以及无法识别的提交不得计入统计。系统必须支持多 provider 记录共存，并根据当前 provider 页签决定统计口径与可用功能。

#### Scenario: All 页签汇总多 provider solve 历史
- **WHEN** 工作区中同时存在 Luogu 与 AtCoder 的标准 `solve(...)` 记录
- **THEN** 系统必须能够在 `All` provider 页签中汇总这些记录的总体统计
- **AND** 系统不得把 `chore(...)` 或 `remove(...)` commit 计入结果

#### Scenario: provider 页签只统计对应来源
- **WHEN** 用户切换到某个单一 provider 页签
- **THEN** 系统必须只使用该 provider 的 `solve(...)` 历史生成统计结果
- **AND** 系统不得把其他 provider 的记录混入当前页签统计

### Requirement: stats 的标签统计必须只使用算法标签
系统 MUST 仅在存在可靠标签体系的 provider 上生成算法标签统计；provider 缺少等价标签体系时，系统必须对标签分布和标签加练建议采用显式降级，而不得伪造标签口径。

#### Scenario: Luogu 页签使用算法标签统计
- **WHEN** 用户位于 Luogu provider 页签
- **THEN** 系统必须继续使用洛谷标签类型字典生成算法标签统计
- **AND** 系统必须保留现有标签分布与标签加练建议体验

#### Scenario: AtCoder 页签缺少可靠算法标签体系
- **WHEN** 用户位于 AtCoder provider 页签
- **THEN** 系统不得伪造与 Luogu 等价的算法标签统计
- **AND** 标签分布区域与标签加练建议必须显示显式降级状态或“不支持”

#### Scenario: All 页签混合多 provider 数据
- **WHEN** 用户位于 `All` provider 页签且当前工作区存在多个 provider 的记录
- **THEN** 系统不得错误混合 provider-specific 标签统计
- **AND** 标签相关区域必须继续采用显式降级或只显示可靠 provider 的独立口径说明

### Requirement: stats 工作台必须以页签方式切换 overview 与 review
系统 MUST 将 stats 概览、题目复习和标签加练实现为同一工作台内的同层模式；`Tab` MUST 作为主模式切换键在三者之间循环，`Esc` MUST 在任一建议模式下先返回 overview，并在 overview 中作为退出当前工作台的允许方式，`q` MUST 在任意模式下直接退出。

#### Scenario: 在三种 stats 模式之间循环切换
- **WHEN** 用户位于 stats 工作台并按下 `Tab`
- **THEN** 系统必须按 `overview -> 题目复习 -> 标签加练 -> overview` 的顺序循环切换
- **AND** 如果目标模式当前没有数据，系统必须显示该模式的空状态而不是拒绝切换

#### Scenario: 从任一建议模式返回 overview
- **WHEN** 用户位于题目复习或标签加练模式并按下 `Esc`
- **THEN** 系统必须返回 overview 模式
- **AND** 系统不得直接退出整个 stats 工作台

### Requirement: stats 页面必须显式展示当前 provider 统计范围
系统 MUST 在 stats 概览、题目复习和标签加练页面中显式展示当前 provider 页签与统计范围，使用户知道当前看到的是 Luogu、AtCoder 还是 All 汇总口径。

#### Scenario: 打开任意 provider 的 overview
- **WHEN** 用户进入某个 provider 页签下的 overview
- **THEN** 页面必须展示当前 provider 与统计范围说明
- **AND** 总体指标必须与该 provider 页签口径保持一致

#### Scenario: 从建议模式返回 overview
- **WHEN** 用户在任一 provider 页签下的题目复习或标签加练模式按下 `Esc`
- **THEN** 系统必须返回同一 provider 页签下的 overview
- **AND** 系统不得隐式跳回其他 provider 页签

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
系统 SHALL 允许用户从 `stats` 命令进入同时包含题目复习与标签加练的建议视图，而不要求切换到另一套命令语义。

#### Scenario: stats 打开双分区建议视图
- **WHEN** 用户在 `stats` 命令中请求建议模式
- **THEN** 系统必须展示题目级复习候选和标签级加练建议两个分区
- **AND** 这些结果必须与同一工作区下的浏览工作台兼容

### Requirement: stats 界面必须以分区概览方式展示核心指标
系统 MUST 在 stats 界面中以清晰分区的概览区域展示核心训练指标，使用户能够快速区分总体数量、当前状态和分布信息，而不是从单一文本段落中自行解析。

#### Scenario: 打开默认 stats 概览
- **WHEN** 用户执行 `aclog stats` 并进入默认统计界面
- **THEN** 页面必须显式展示工作区上下文与统计范围
- **AND** 页面必须将总体指标与结果/难度/标签分布分区展示

### Requirement: stats 与 review 视图必须共享主题但保持模式可辨识
系统 MUST 让 stats 概览模式、题目复习模式和标签加练模式共享统一终端主题，同时清晰区分三种模式，避免用户混淆当前界面的操作目标。

#### Scenario: 从概览切换到题目复习或标签加练
- **WHEN** 用户在 stats 界面中切换到任一建议模式
- **THEN** 页面必须明确显示当前处于题目复习模式或标签加练模式
- **AND** 建议列表、建议详情和可钻取动作必须与统计概览形成清晰区分

### Requirement: stats 页面必须显式展示帮助与钻取入口
系统 MUST 在 stats 页面中显式展示当前支持的钻取入口和帮助提示，使用户知道如何进入题目复习、标签加练、文件浏览、题目浏览或退出当前页面。

#### Scenario: stats 页面显示当前操作
- **WHEN** 用户停留在 stats 概览、题目复习或标签加练页面
- **THEN** 页面必须显示当前模式下可用的核心动作
- **AND** 帮助显示不得改变当前统计结果或建议结果

#### Scenario: stats 保留跨工作台快捷跳转
- **WHEN** 用户在 stats 工作台中触发文件浏览、题目浏览，或从标签加练进入带标签过滤的题目视图
- **THEN** 系统必须继续支持直接跳转到对应的浏览工作台
- **AND** 这些快捷键必须与 `Tab` 的同层模式切换语义区分开
