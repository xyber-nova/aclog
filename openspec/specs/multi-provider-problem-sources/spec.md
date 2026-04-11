## Purpose

为 `aclog` 定义统一的多 provider 题目标识、provider-aware API 路由与 AtCoder 上下文建模语义，使题目元数据、提交记录和缓存都能在 Luogu / AtCoder 共存的前提下保持一致。

## Requirements

### Requirement: 系统必须支持多 provider 的统一题目标识
系统 MUST 使用单字符串全局题目标识作为跨 provider 的唯一题目键，并至少支持 `luogu:<raw-id>` 与 `atcoder:<task-id>` 两种格式。

#### Scenario: 为不同 provider 生成全局题目标识
- **WHEN** 系统识别出一个 Luogu 题目 `P1001` 或一个 AtCoder 题目 `abc350_a`
- **THEN** 系统必须分别生成 `luogu:P1001` 与 `atcoder:abc350_a` 作为内部唯一题目标识
- **AND** 系统不得继续把不同 provider 的原始题号直接视为同一命名空间

#### Scenario: 旧格式 Luogu 记录进入多源读模型
- **WHEN** 系统解析到旧格式 `solve(P1001): ...` 且记录来源可判定为 Luogu
- **THEN** 系统必须在内存读模型中将其归一为 `luogu:P1001`
- **AND** 系统不得要求用户先重写旧历史才能参与多源聚合

### Requirement: 系统必须提供 provider-aware 的题目与提交读取接口
系统 MUST 能够基于全局题目标识把题目元数据和用户提交请求分发到对应 provider，并对调用方暴露统一的题目/提交模型。

#### Scenario: Luogu 全局题目标识走 Luogu provider
- **WHEN** 调用方向系统请求 `luogu:P1001` 的题目元数据或用户提交记录
- **THEN** 系统必须把该请求分发到 Luogu provider
- **AND** 返回结果必须仍使用统一模型和同一全局题目标识

#### Scenario: AtCoder 全局题目标识走 AtCoder provider
- **WHEN** 调用方向系统请求 `atcoder:abc350_a` 的题目元数据或用户提交记录
- **THEN** 系统必须把该请求分发到 AtCoder provider
- **AND** 调用方不得需要理解 AtCoder Problems API 的具体线形或字段差异

### Requirement: AtCoder 文件名解析必须兼容常见竞赛任务写法
系统 MUST 将 AtCoder 竞赛型任务文件名同时解析为带下划线与不带下划线的两种常见写法，并归一化为同一个 task-id。

#### Scenario: 竞赛任务支持紧凑文件名
- **WHEN** 系统看到 `abc447a.cpp` 或 `abc447_a.cpp` 这样的 AtCoder 文件名
- **THEN** 系统必须将两者都识别为同一个 task-id `abc447_a`
- **AND** 系统内部仍然必须使用统一的 `atcoder:abc447_a` 作为全局题目标识

#### Scenario: 非竞赛型 AtCoder 任务仍保持下划线格式
- **WHEN** 系统看到 `typical90_001.py`、`dp_a.cpp` 或 `math_and_algorithm_ak.cpp` 这类文件名
- **THEN** 系统必须继续按照原有下划线格式解析为对应 AtCoder task-id
- **AND** 系统不得把这些任务误判为竞赛型 `abc/arc/agc/ahc` 任务

### Requirement: AtCoder 题目元数据必须保留比赛上下文
系统 MUST 在 AtCoder 题目元数据中保留可选的比赛上下文字段，以便终端界面和浏览/统计工作台展示题目所属比赛。

#### Scenario: AtCoder 元数据包含比赛信息
- **WHEN** 系统成功读取某个 AtCoder task 的题目信息
- **THEN** 元数据必须包含 `source = AtCoder`
- **AND** 若 provider 能提供比赛信息，系统必须把该比赛名或比赛 ID 存入可选 `contest` 字段

#### Scenario: 比赛信息暂时缺失
- **WHEN** AtCoder provider 当前无法为某个 task 返回比赛信息
- **THEN** 系统仍然必须返回该题的统一元数据
- **AND** `contest` 字段可以为空而不是阻断整个题目读取

### Requirement: provider 相关缓存必须以全局题目标识隔离
系统 MUST 基于全局题目标识隔离题目元数据缓存和 provider 读取状态，避免不同 provider 的同名原始题号相互覆盖。

#### Scenario: 两个 provider 存在相同原始题号片段
- **WHEN** 系统先后缓存 `luogu:P1001` 与某个原始后缀相同的其他 provider 题目
- **THEN** 两者必须写入彼此独立的缓存键或缓存文件
- **AND** 读取其中任一题目时不得命中另一 provider 的缓存内容
