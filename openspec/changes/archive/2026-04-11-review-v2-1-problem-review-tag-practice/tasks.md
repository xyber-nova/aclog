## 1. Review 数据模型与配置

- [x] 1.1 扩展 review 领域模型，将单一 `ReviewCandidate` / `review_candidates` 拆为 `problem_reviews` 与 `tag_practice_suggestions`
- [x] 1.2 在配置读取与默认配置生成中新增 `review_problem_interval_days`、`practice_tag_window_days`、`practice_tag_target_problems`，并为旧配置提供默认回退
- [x] 1.3 更新相关序列化结构与测试辅助对象，使 app / UI / JSON 输出能够消费新的双分区 review 数据

## 2. Review 聚合逻辑

- [x] 2.1 重构 `src/domain/stats.rs` 中的 review 生成逻辑，实现题目复习候选的“状态直通 + 固定间隔 + 优先级”规则
- [x] 2.2 实现标签加练建议的样本量统计、优先级排序和解释文案，并确保标签不再输出 `weakness` / `stale` / `retry` 语义
- [x] 2.3 调整 `stats` app 的建议输出路径，让 `--review` TUI 和 `--review --json` 共用新的双分区结果

## 3. Stats 工作台交互

- [x] 3.1 将 stats TUI 从 `overview / review` 两态扩展为 `overview / 题目复习 / 标签加练` 三态循环
- [x] 3.2 为题目复习和标签加练分别实现列表、详情、空状态与页脚帮助文案
- [x] 3.3 保持 `Enter` 深链能力：题目复习进入题目时间线，标签加练进入带标签过滤的题目浏览视图

## 4. 验证与回归测试

- [x] 4.1 为题目复习规则补充单元测试，覆盖立即复习、间隔复习、缺少 `submission_time` 与优先级排序
- [x] 4.2 为标签加练规则补充单元测试，覆盖样本不足、近期不稳信号说明与排序规则
- [x] 4.3 更新 app / TUI 测试，覆盖双分区 review 输出、三态切换、空状态和 JSON 结构
- [x] 4.4 运行 `cargo fmt --check`、`cargo check`、`cargo test` 验证整体验证通过
