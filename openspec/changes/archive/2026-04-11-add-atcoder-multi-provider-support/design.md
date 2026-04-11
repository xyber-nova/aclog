## Context

`aclog` 当前围绕 Luogu 单一 provider 组织数据模型、远端 API 调用、commit 协议和终端工作台。题目标识默认是裸题号，`sync`、`record`、`browser`、`stats` 以及共享历史索引都把该题号当作唯一键。

这次变更需要同时引入 AtCoder Problems 非官方 API、兼容既有 Luogu 历史、扩展终端工作台，并且保持“`jj` 历史即事实源”的核心语义不变。设计必须处理三类复杂度：

- 数据模型从单 provider 升级为多 provider，同时不能破坏既有 Luogu 历史读取。
- UI 要加入来源与比赛上下文，但不能把 `sync` / selector 的轻量交互变成多层工作台。
- `stats` 不能假设所有 provider 都具备与 Luogu 等价的标签体系，需要显式降级。

## Goals / Non-Goals

**Goals:**
- 为 Luogu 与 AtCoder 建立统一的全局题目标识与 provider 分发模型。
- 新生成记录统一写入全局题目标识，并兼容读取旧 Luogu 裸题号历史。
- 在 `sync` / `record` 中展示 provider 与比赛上下文，在 `browser` / `stats` 中加入 provider 页签。
- 让 `stats` 对多 provider 采用稳定、可解释的降级策略，而不是混合出错误标签口径。

**Non-Goals:**
- 首版不新增 `contests` 根视角，也不实现“按比赛聚合回看整场比赛”的工作台。
- 首版不自动重写现有 Luogu 历史以迁移到新协议。
- 首版不为未知来源旧记录提供完整的 provider 页签能力，只要求显式降级而不是静默误归类。
- 首版不尝试为 AtCoder 构造与 Luogu 完全等价的算法标签体系。

## Decisions

### 1. 统一使用单字符串全局题目标识
采用 `luogu:P1001`、`atcoder:abc350_a` 形式作为内部唯一题目键，并把它直接写入新 commit 头部。

原因：
- 能兼容当前大量把 `problem_id` 当字符串处理的领域模型、索引和筛选逻辑。
- 比“单独 provider 字段 + 裸 problem_id”更容易在历史协议、CLI 过滤和序列化缓存里保持唯一性。
- 比“只把 provider 放在 `Source` 字段”更稳，避免不同 provider 共享相似原始题号时歧义。

替代方案：
- `provider + raw_id` 分字段建模：语义更清晰，但会引发更大范围的接口/序列化改动；可在后续内部再逐步演化，不作为首版落地方式。
- 继续使用裸题号：与多 provider 冲突风险过高，不可接受。

### 2. provider 分发层保留统一接口，对外仍传全局 ID
`ProblemProvider` 与 `src/api/mod.rs` 保持“统一入口”形式，调用方传全局 ID，provider dispatcher 在 API 层拆分 provider 并路由到 Luogu / AtCoder client。

原因：
- workflow 层无需理解 provider 分发细节。
- fake deps 与集成测试可以继续通过统一接口注入数据。
- 降低 `sync` / `record` / `stats` workflow 的改动面。

替代方案：
- 把 provider-aware 类型一路上推到所有 app 层接口：长期更规范，但当前会放大工作量，且与首版目标不成比例。

### 3. AtCoder 比赛信息建模为可选元数据，不进入主键
AtCoder 使用 task 级全局 ID 作为主键，`contest` 仅作为 `ProblemMetadata` 和 UI 展示的可选上下文。

原因：
- task 已经能够唯一标识 AtCoder 题目。
- 比赛字段更适合作为浏览与理解上下文，而不是索引主键。
- 避免把“比赛视角”提前固化到所有索引与记录协议中。

替代方案：
- 把比赛写进主键：会放大协议与索引复杂度，但对首版功能收益有限。
- 完全不保留比赛字段：会让 AtCoder UI 丢失最关键的人类可读上下文。

### 4. 历史兼容使用“读时归一化”，不做迁移写回
旧 Luogu 裸题号历史通过 `Source: Luogu` 或等价线索在解析时归一化为 `luogu:<raw>`；无法可靠判断 provider 的旧记录标记为未知来源并降级。

原因：
- 不破坏现有仓库历史，也不强迫用户先运行迁移。
- 继续遵守“`jj` 历史即事实源”，避免引入旁路数据库。
- 能把兼容复杂度限制在 parser / index 层，而不是扩散到每个 workflow。

替代方案：
- 自动 rewrite 旧历史：风险高、侵入性强，不适合作为首版。
- 无法判断的旧记录直接猜成 Luogu：会把多 provider 聚合做错，不可接受。

### 5. UI 分层：`sync` / selector 补信息，`browser` / `stats` 加 provider 页签
`sync` 与 `record` 选择器保持轻量单工作流结构，只新增来源与比赛展示；`browser` 与 `stats` 引入 `Luogu / AtCoder / All` provider 页签。

原因：
- `sync` 和 selector 的核心任务是快速做决策，不适合再套一层 provider 模式切换。
- `browser` 与 `stats` 本来就是工作台，更适合承载 provider 页签。
- 保留现有 files/problems、overview/review 等核心模式，避免一次性重写所有交互习惯。

替代方案：
- 所有 TUI 都引入 provider 页签：交互负担过大。
- 完全不引入 provider 页签，只靠筛选：对多源工作区的可发现性不足。

### 6. `stats` 采用 provider-aware 降级
总体统计、题目复习可支持多 provider；标签分布和标签加练仅在存在可靠标签体系的 provider 上启用。AtCoder 与 All 页签首版对标签相关区域显示显式降级/不支持。

原因：
- Luogu 标签体系与 AtCoder 现有可用数据并不对等。
- 强行混合会生成看似完整但实际错误的统计口径。
- 显式降级比隐式遗漏更可理解，也便于未来为 AtCoder 单独补全标签或题型体系。

替代方案：
- 只让 `stats` 看 Luogu：会破坏“多 provider 共存”的目标。
- 混合统计但让缺标签显示 `-`：用户难以分辨哪些指标是可靠的。

## Risks / Trade-offs

- [旧历史 provider 无法可靠识别] → 通过显式 `unknown:*` 或等价降级状态保留记录，但不让其进入精确 provider 页签聚合。
- [AtCoder Problems 是非官方 API] → 把 provider 波动、限流和降级封装在 AtCoder client 内；UI 只消费统一模型与失败语义。
- [全局 ID 进入 commit 头部后人类可读性略降] → 在 UI 与详情页默认展示 provider、raw id、title 和 contest，而不是直接暴露内部 key。
- [browser/stats 引入 provider 页签会增加状态机复杂度] → 维持两层但不再继续添加 contests 根视角，把焦点重置与空状态作为明确规则实现。
- [缓存键切到全局 ID 后会与旧缓存文件并存] → 允许旧缓存自然失效，不强制迁移；读路径优先新键格式。

## Migration Plan

1. 先引入全局题目标识解析与 provider dispatcher，保证新旧 Luogu 记录能同时被读模型接受。
2. 扩展 commit parser / builder、共享索引与领域行模型，让全局 ID 在 history、browser、stats 中贯通。
3. 加入 AtCoder provider client 与缓存，打通 `sync` / `record` workflow 的多源元数据和 submission 拉取。
4. 更新 `sync` / selector 的来源和比赛展示，再实现 browser/stats provider 页签。
5. 为 stats 增加 provider-aware 降级逻辑，最后补齐 fake deps、workflow 测试与混合历史回归。

回滚策略：
- 若 AtCoder provider 不稳定，可暂时禁用 `atcoder:*` 解析与 dispatcher 分支，同时保留 Luogu 新全局 ID 方案。
- 若 UI provider 页签存在问题，可先回退到只暴露 `All` 或单 provider 视图，而不回退底层全局 ID 模型。

## Open Questions

当前没有阻断首版实现的开放问题。默认实现取值如下：

- 未知来源旧记录在读模型中使用 `unknown:<raw>` 形式保留原始题目标识，并从精确 provider 页签聚合中排除。
- AtCoder 题目难度首版直接使用 AtCoder Problems 可提供的值；缺失时统一回退到 `-`。
- browser 以 `Tab` 切 provider 页签、`f` / `p` 切 files/problems 根视角；stats 以 `Tab` 切 provider 页签，`o` / `r` / `g` 分别切 overview / problem review / tag practice。
