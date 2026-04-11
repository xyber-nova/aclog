## MODIFIED Requirements

### Requirement: 系统必须提供记录浏览工作台
系统 MUST 提供一个面向本地记录历史的浏览工作台，并至少支持 provider 页签、文件视角、题目视角和时间线视角四类浏览模式。provider 页签首版必须至少覆盖 `Luogu`、`AtCoder` 和 `All`。

#### Scenario: 在 Luogu 页签查看当前记录状态
- **WHEN** 用户进入记录浏览工作台并切换到 `Luogu` provider 页签
- **THEN** 系统必须只展示来源为 Luogu 的文件视角或题目视角结果
- **AND** 每个文件必须只展示最新的一条标准 `solve(...)` 记录作为当前状态

#### Scenario: 在 AtCoder 页签查看当前记录状态
- **WHEN** 用户进入记录浏览工作台并切换到 `AtCoder` provider 页签
- **THEN** 系统必须只展示来源为 AtCoder 的文件视角或题目视角结果
- **AND** 题目详情必须能够展示比赛上下文

#### Scenario: 在 All 页签查看混合记录状态
- **WHEN** 用户进入记录浏览工作台并切换到 `All` provider 页签
- **THEN** 系统必须允许 Luogu 与 AtCoder 记录在同一工作台中共同浏览
- **AND** 系统不得因为来源不同而重新定义文件时间线或题目时间线语义

### Requirement: 浏览工作台必须支持多条件筛选
浏览工作台 MUST 支持至少按 provider、题号、文件名、标签、难度、结果和时间窗口筛选记录。

#### Scenario: 使用 provider 单独筛选
- **WHEN** 用户在浏览工作台中指定 provider 筛选
- **THEN** 系统必须只展示该 provider 下的记录集合
- **AND** provider 筛选必须同时作用于根视角与时间线视图

#### Scenario: provider 与其他条件组合筛选
- **WHEN** 用户在浏览工作台中同时指定 provider、题号或文件名等多个筛选条件
- **THEN** 系统必须按这些条件的交集筛选结果
- **AND** 系统不得在未说明的情况下放宽过滤条件

### Requirement: 浏览工作台必须显式展示当前视角与筛选摘要
系统 MUST 在记录浏览工作台中显式展示当前 provider 页签、根视角、时间线入口和筛选摘要，使用户能够一眼理解当前看到的是哪一类记录集合。

#### Scenario: 打开 provider 根视角工作台
- **WHEN** 用户进入记录浏览工作台的任一 provider 页签下的文件视角或题目视角
- **THEN** 页面必须展示当前 provider、当前根视角标识
- **AND** 页面必须展示当前生效的题号、文件名、结果、难度、标签或时间窗口筛选摘要

#### Scenario: 从统计或建议跳入时间线
- **WHEN** 用户从统计或复习建议入口跳转到某个题目或文件时间线
- **THEN** 页面必须显式展示当前 provider、当前时间线对象及返回路径提示

### Requirement: 浏览工作台必须以页签方式切换同层模式
系统 MUST 将浏览工作台中的 provider 视图实现为第一层页签，并保留文件/题目作为第二层根视角；`Tab` MUST 作为 provider 页签的主切换键，根视角切换必须使用另一明确键位或等价的清晰模式切换语义，`Esc` MUST 作为返回上一层的主键，`q` MUST 作为退出工作台的主键。

#### Scenario: 在 provider 页签之间切换
- **WHEN** 用户位于浏览工作台根视图并按下 `Tab`
- **THEN** 系统必须在 `Luogu -> AtCoder -> All -> Luogu` 之间循环切换
- **AND** 当前焦点必须重置到新页签下可用的首个候选项或空状态

#### Scenario: 从时间线返回当前 provider 根视图
- **WHEN** 用户位于文件时间线或题目时间线并按下 `Esc`
- **THEN** 系统必须返回对应 provider 页签下的根视图
- **AND** 历史上的返回快捷键可以保留为兼容别名，但不得继续作为主提示键位

## ADDED Requirements

### Requirement: 浏览工作台详情必须展示来源与比赛上下文
系统 MUST 在文件视角、题目视角和时间线视角的详情区域中展示当前记录的 provider 信息，并在适用时展示比赛上下文。

#### Scenario: 查看 Luogu 详情
- **WHEN** 用户在 Luogu 页签中移动当前选中项
- **THEN** 详情区域必须展示来源为 Luogu
- **AND** 系统可以不显示比赛字段

#### Scenario: 查看 AtCoder 详情
- **WHEN** 用户在 AtCoder 页签中移动当前选中项
- **THEN** 详情区域必须展示来源为 AtCoder
- **AND** 若当前题目存在比赛信息，详情区域必须展示该比赛字段
