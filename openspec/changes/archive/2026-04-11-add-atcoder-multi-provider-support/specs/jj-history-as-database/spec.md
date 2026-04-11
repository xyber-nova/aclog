## ADDED Requirements

### Requirement: 结构化训练记录必须支持多 provider 全局题目标识
系统 MUST 允许本地 `jj` 历史中的结构化训练记录使用多 provider 全局题目标识作为题目键，并继续把这些 commit 作为训练事实源。

#### Scenario: 新记录使用全局题目标识写入 commit
- **WHEN** 系统为 Luogu 或 AtCoder 题目创建新的 `solve(...)`、`chore(...)` 或 `remove(...)` 记录
- **THEN** 记录头部必须写入对应的全局题目标识
- **AND** 该 commit 自身必须继续承担训练事实源职责

#### Scenario: 旧格式 Luogu 记录进入统一读模型
- **WHEN** 系统读取旧格式 Luogu 历史记录，其头部题号仍为裸题号
- **THEN** 共享读模型必须能够把它归一为 Luogu 全局题目标识
- **AND** 系统不得要求用户先重写历史才能继续浏览、维护或统计这些记录

### Requirement: 无法可靠识别 provider 的旧记录必须显式降级
当系统读取到旧格式 `solve(...)` 记录但无法从协议内容中可靠推断 provider 时，系统 MUST 将其视为显式降级对象，而不得把它静默混入某个已知 provider 的聚合结果。

#### Scenario: 旧记录缺少可判定来源
- **WHEN** 系统解析某条旧格式 `solve(...)` 记录，且无法从头部或 `Source` 字段可靠判定 provider
- **THEN** 系统必须将该记录标记为未知来源或等价的降级状态
- **AND** 该记录不得参与多 provider 的精确 provider 页签聚合

#### Scenario: 未知来源记录仍保留事实源语义
- **WHEN** 某条记录被判定为未知来源
- **THEN** 系统仍然必须保留其作为本地训练历史的一部分
- **AND** 系统不得因为 provider 无法判定就把该记录从本地事实库中删除
