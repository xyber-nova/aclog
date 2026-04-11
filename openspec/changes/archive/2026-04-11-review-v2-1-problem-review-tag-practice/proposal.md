## Why

当前 `stats --review` 会把题目复习候选与标签级建议混在同一套 `stale / retry / weakness` 语义里，导致“该回看做过的题”和“这个方向该多做些题”之间的界线不清，也会让标签出现不准确的“需要复习”表述。结合实际使用经验，题目复习更适合遵循 2~4 周的稳定节奏，而标签层更应该表达覆盖不足与补样本建议。

## What Changes

- 将 review 输出拆成两个独立分区：`problem_reviews` 和 `tag_practice_suggestions`
- 将题目复习规则改为“状态直通 + 固定间隔到期”模型，默认复习间隔为 21 天
- 将标签建议改为“建议多做题”的覆盖不足信号，不再把标签命名为 `stale`、`retry` 或 `weakness`
- 为 stats review JSON 输出和终端工作台引入双分区结构，并保留到 browser 的钻取能力
- 为题目复习间隔与标签样本阈值新增少量配置项，并在缺失时回退到稳定默认值

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `training-review-suggestions`: 将建议语义拆分为题目复习与标签加练，替换现有标签级 `weakness` 语义
- `stats-command`: 调整 stats review 输出结构、配置项和工作台模式，使其支持双分区 review
- `terminal-ui-experience`: 更新 stats 工作台的模式切换与详情呈现，保持统一终端交互语义

## Impact

- Affected code: `src/domain/stats.rs`, `src/app/stats.rs`, `src/ui/terminal/stats.rs`, `src/config.rs` 及相关测试
- Public interfaces: `StatsDashboard` 与 review JSON 输出结构将从单一候选列表升级为双分区对象
- User-facing behavior: stats review 页会从单一列表改为“题目复习 / 标签加练”两个分区，并采用新的解释文案
