## Context

当前 `stats --review` 由 `src/domain/stats.rs` 中的单一候选列表驱动，把题目级 `stale / retry` 候选与标签级 `weakness` 候选直接混排。这样虽然实现简单，但会把“做过的题该回看”和“这个方向样本太少，建议多做题”混成同一语义，也让标签承担了并不准确的“复习对象”角色。

这次变更会同时影响领域聚合、stats app 输出、终端工作台以及配置读取，因此需要在实现前明确数据结构、排序原则和界面交互。项目仍需遵守“`jj` as database”原则，所有建议都必须完全由本地 `solve(...)` 历史和训练字段派生，且 `cargo test` 需要保持离线可运行。

## Goals / Non-Goals

**Goals:**
- 将 review 语义拆分为题目复习和标签加练两条独立能力
- 用固定间隔加状态直通规则生成题目复习候选，默认间隔为 21 天
- 用最近样本量不足生成标签加练建议，并允许附带最近不稳信号作为说明
- 让 stats TUI 在同一工作台内清晰展示 `overview / 题目复习 / 标签加练` 三种模式
- 为 review 行为新增少量配置项，并在缺失时稳定回退到默认值

**Non-Goals:**
- 不引入 FSRS、SM-2 或其他记忆模型
- 不新增独立的 review commit、review 数据库或其他事实源
- 不改变 browser 工作台的根视图结构，只通过现有 query 参数深链过去
- 不把标签建议升级为“标签薄弱度排名”或“标签复习计划”

## Decisions

### Decision: review 输出改为双分区对象，而不是继续扩展单一候选类型

`StatsDashboard` 和 `stats --review --json` 将从单一 `review_candidates` 列表拆成两个集合：
- `problem_reviews`
- `tag_practice_suggestions`

这样可以避免在单个枚举里同时表达题目与标签两种对象，也让 UI 与 JSON 都能直接体现“题目复习”和“标签加练”的语义边界。

Alternatives considered:
- 继续使用单一 `ReviewCandidate` 并添加 `kind = problem_review | tag_practice`
  - Rejected: 仍然会让 JSON 和 UI 消费方承受大量条件分支，且不利于分别排序与分别展示空状态。

### Decision: 题目复习使用“状态直通 + 固定间隔”模型

题目候选按题号聚合历史后，仅依据该题最近状态和最近少量历史信号决定是否进入复习：
- 最新非 AC、`Mistakes`、`Confidence=low` 直接进入
- 否则按 `review_problem_interval_days` 计算是否到期
- 再用最近 3 次内的额外非 AC 次数与到期间隔阶梯提升优先级

这个模型与用户稳定的 2~4 周回看习惯一致，也能避免旧的“历史里至少两次非 AC 就长期滞留”问题。

Alternatives considered:
- 直接沿用现有 `stale / retry` 规则并调整阈值
  - Rejected: 仍然无法消除旧失败记录对当前健康题目的长期污染。
- 引入 FSRS
  - Rejected: 当前没有显式复习事件与打分，不适合直接套用间隔重复模型。

### Decision: 标签建议只做覆盖不足提示，并用最近窗口与全历史共同排序

标签层只表达“建议多做题”，不再输出 `weakness`。实现上会按算法标签统计：
- 最近 `practice_tag_window_days` 内涉及的不同题目数
- 全历史涉及的不同题目数
- 最近窗口内非 AC 或 `Confidence=low` 的计数

进入建议的条件是最近样本不足，排序优先看缺口，再用全历史样本量做次排序。最近不稳信号只作为解释文本增强，不改变标签的基本语义。

Alternatives considered:
- 继续按非 AC 和低熟练度累计做标签得分
  - Rejected: 会把“做得多但偶尔失误”误判成更弱，也无法区分“表现差”和“样本少”。

### Decision: stats 工作台扩展为三态循环，而不是在 review 内再做二级切换

终端工作台维持同层模式切换：
- `overview`
- `problem review`
- `tag practice`

`Tab` 在三者间循环，`Esc` 在任一建议模式下先回 overview，`q` 直接退出。每个建议模式都保留左侧列表 + 右侧详情的阅读结构，并通过 `Enter` 深链到 browser。

Alternatives considered:
- 保持 overview/review 两态，并在 review 页内部再切子页签
  - Rejected: 容易让帮助文案和键位语义分层不清，也会让 `Esc` 的返回行为更难解释。

### Decision: review 配置独立于概览时间窗口

新增配置：
- `review_problem_interval_days`
- `practice_tag_window_days`
- `practice_tag_target_problems`

stats 的 `--days` 继续服务于概览统计；review 建议使用自己的配置，不把概览窗口直接等同于复习间隔。这能让“我想看最近 7 天概览”和“我希望按 21 天节奏回看题目”同时成立。

Alternatives considered:
- 让 `--days` 同时控制概览与 review
  - Rejected: 一个参数同时表达“统计窗口”和“复习周期”会让语义持续冲突。

## Risks / Trade-offs

- [Risk] review JSON 结构从单一数组变为对象，可能影响依赖旧输出的调用方
  → Mitigation: 在变更说明中明确这是一次 review 输出升级，并在测试中覆盖新的 JSON 结构

- [Risk] review 模式从两态扩展为三态后，终端帮助和导航可能更复杂
  → Mitigation: 维持统一的 `Tab / Esc / q / Enter` 语义，并让标题与页脚明确展示当前模式

- [Risk] 标签建议只看样本不足，可能漏掉“样本够但近期明显不稳”的标签聚类提醒
  → Mitigation: 该类信号保留在题目复习列表里处理；后续如需新增“标签表现分析”，以新能力而不是混入标签加练语义的方式扩展

- [Risk] 缺少 `submission_time` 的记录无法参与固定间隔复习，可能让少量老记录不再被常规 review 捕获
  → Mitigation: 仍保留状态直通规则，并在实现中只对时间缺失的记录关闭“到期复习”分支

## Migration Plan

1. 更新领域模型与 review 生成逻辑，保留 stats 概览逻辑不变
2. 扩展配置读取与默认配置生成，确保旧配置文件在缺失新字段时仍能正常运行
3. 重构 stats app 的 review JSON 输出与 TUI 数据流
4. 更新相关单元测试、app 集成测试与 TUI 行为测试
5. 如发现外部调用方依赖旧 JSON，提供后续兼容方案；本次变更不引入双格式输出

## Open Questions

None.
