## ADDED Requirements

### Requirement: stats 命令必须提供独立的本地统计入口
系统必须提供 `aclog stats` 命令，用于在指定工作区中展示基于本地历史的做题统计，而不改变现有 `sync` 流程。

#### Scenario: 使用默认工作区运行 stats
- **WHEN** 用户执行 `aclog stats`
- **THEN** 系统必须以当前目录作为工作区读取本地统计数据
- **AND** 系统必须进入统计界面而不是执行 `sync`

#### Scenario: 使用显式工作区运行 stats
- **WHEN** 用户执行 `aclog stats --workspace /path/to/workspace`
- **THEN** 系统必须从该路径加载工作区并读取统计数据

### Requirement: stats 命令必须基于本地 solve 历史生成统计
系统必须以当前工作区本地 `jj` 历史中的 `solve(...)` commit 作为做题统计的主体数据源；`chore(...)`、`remove(...)` 以及无法识别的提交不得计入统计。为完成算法标签分类，系统可以读取本地或远端的标签类型字典。

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
系统必须仅使用算法标签生成标签统计；来源、时间、地区、特殊题目等非算法标签不得进入统计结果，但 `ProblemMetadata.tags` 和 `solve(...)` commit 中的原始标签不得被截断。

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
系统必须同时展示按题号去重后的唯一题目统计，以及全部 `solve(...)` 记录统计，以区分“做过多少题”和“做题活动量”。

#### Scenario: 同一道题存在多条 solve 记录
- **WHEN** 某个题号在本地历史中出现多条 `solve(...)` commit
- **THEN** 系统必须在“全部记录”口径中统计全部这些记录
- **AND** 系统必须在“唯一题目”口径中只将该题统计一次

#### Scenario: 唯一题目统计需要当前状态
- **WHEN** 系统为某个题号计算唯一题目统计
- **THEN** 系统必须使用该题最新的一条 `solve(...)` 记录作为当前状态

### Requirement: stats 界面必须展示首版概览指标
统计界面必须至少展示唯一题目数、`solve` 记录数、AC 与非 AC 情况、按结果分布、按难度分布以及按算法标签分布。

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

### Requirement: stats 界面必须提供明确空状态
当当前工作区没有任何可识别的 `solve(...)` 提交时，统计界面必须展示空状态，并允许用户直接退出。

#### Scenario: 工作区没有 solve 历史
- **WHEN** 用户进入 `aclog stats` 且当前工作区没有任何可识别的 `solve(...)` commit
- **THEN** 系统必须明确提示当前工作区还没有已记录的做题提交
- **AND** 系统必须提供退出方式
