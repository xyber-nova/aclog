## 1. Repository Contract and Actor Scaffolding

- [x] 1.1 为 Tokio 补充 `sync` feature，并在 `src/vcs/` 下新增 actor 相关模块与基础类型
- [x] 1.2 定义语义化仓库 trait，覆盖工作区校验、变更检测、记录索引加载、revision 解析、tracked file 判断和记录写入等核心能力
- [x] 1.3 实现 `JjRepoActorHandle`、请求消息枚举和基于 `mpsc` / `oneshot` 的 actor 循环
- [x] 1.4 将现有 `jj-lib` 只读逻辑与 `jj` CLI 写逻辑收编到 actor 内部分发路径，同时保持读写分离约束

## 2. Live Deps and Command Wiring

- [x] 2.1 重构 `LiveDeps`，让其持有命令级仓库 actor handle 而不是继续作为零大小 live adapter
- [x] 2.2 更新顶层 `run_*` 命令入口，使每次命令为目标工作区构造一份 live deps 并复用到整条 workflow
- [x] 2.3 从 app 层移除对低层 `vcs` 细节的直接依赖，统一改为调用语义化仓库接口

## 3. Workflow Migration

- [x] 3.1 更新 `app/support` 的记录索引加载、revset 解析和 tracked file 判断逻辑，改走新的仓库接口
- [x] 3.2 迁移 `sync`、`record bind/rebind/edit/list/show`、浏览和统计 workflow 到 actor-backed 仓库边界
- [x] 3.3 收缩 `src/vcs/mod.rs` 的对外导出，只保留新的语义化仓库入口和必要的内部实现边界

## 4. Tests, Specs, and Cleanup

- [x] 4.1 更新 `tests/support/` 中的 fake 仓库依赖，使其实现新的语义化仓库 trait 而无需启动 live actor
- [x] 4.2 为 actor 增加 FIFO 顺序、响应回传和写后读可见性的单元测试
- [x] 4.3 更新真实 `jj` 集成测试，验证 actor-backed live 路径下的创建、重写与后续历史读取
- [x] 4.4 运行 OpenSpec 校验与现有 Rust 测试套件，确认 change apply-ready 且行为无回归
