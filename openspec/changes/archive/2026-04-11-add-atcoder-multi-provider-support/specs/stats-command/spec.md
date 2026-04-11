## MODIFIED Requirements

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
系统 MUST 将 stats 工作台扩展为“provider 页签 + 视图模式”的两层结构；provider 页签首版必须至少覆盖 `Luogu`、`AtCoder` 和 `All`，`Tab` MUST 作为 provider 页签的主切换键，overview / 题目复习 / 标签加练的模式切换必须继续存在并采用清晰可见的操作语义。

#### Scenario: 在 provider 页签之间循环切换
- **WHEN** 用户位于 stats 工作台并按下 `Tab`
- **THEN** 系统必须按 `Luogu -> AtCoder -> All -> Luogu` 的顺序循环切换 provider 页签
- **AND** 切换后必须重置当前 provider 下不可用的焦点状态

#### Scenario: AtCoder 页签仍可进入题目复习
- **WHEN** 用户位于 AtCoder provider 页签
- **THEN** 系统必须继续允许进入 overview 与题目复习模式
- **AND** 若标签加练模式对当前 provider 不适用，系统必须显示显式空状态或降级提示而不是崩溃退出

## ADDED Requirements

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
