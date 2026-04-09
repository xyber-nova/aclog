## Context

当前 `aclog` 已经明确采用 `jj-lib` 负责只读、`jj` CLI 负责写操作的读写分离策略，但仓库边界仍然偏低层：`src/vcs/read.rs` 与 `src/vcs/write.rs` 分别暴露工具级函数，`src/app/deps.rs` 中的 `RepoGateway` 也直接把这些低层操作抛给上层 workflow。结果是：

- 应用层仍然需要理解“现在是读还是写”“应该调用哪组仓库函数”。
- 仓库语义没有被集中建模，`load_record_index` 之类的组合逻辑散落在 `app/support` 中。
- 现在的命令流虽然基本串行，但未来一旦出现更复杂的交互、后台刷新或重构，仓库访问顺序可能在不经意间被打乱。

这次改动不会改变用户可见命令，而是要把仓库访问重构成稳定的统一入口，并把顺序保证从“碰巧串行”升级为“设计上串行”。在当前实现里，这个入口在同一 `aclog` 进程内对同一工作区保持唯一，避免上层因为重复构造依赖而意外生成多个 live actor。

## Goals / Non-Goals

**Goals:**
- 用统一的仓库语义接口替换当前偏底层的 `RepoGateway`。
- 在单次 CLI 命令内通过一个轻量级 actor 串行化所有 live `jj` 访问。
- 继续保持内部读写分离：读取优先 `jj-lib`，写入使用 `jj` CLI。
- 让 workflow 层不再显式区分 `jj-lib` 与 `jj` CLI。
- 保持测试可替换性，让 fake 仓库依赖继续成立。

**Non-Goals:**
- 不引入常驻 daemon 或全局 actor registry。
- 不支持同一进程内的多工作区并发编排。
- 不引入第三方 actor 框架。
- 不修改现有 CLI 命令面、参数语义或用户输出格式。
- 不把问题元数据提供者或输出分发器并入仓库 actor。

## Decisions

### 1. 将低层 `RepoGateway` 重构为语义化仓库接口

新的仓库接口将围绕当前 workflow 真正需要的能力建模，而不是继续暴露一组“底层仓库原语”。首版接口固定为：

- `ensure_workspace()`
- `detect_working_copy_changes()`
- `load_record_index()`
- `resolve_revision(revset)`
- `is_tracked_file(path)`
- `create_commits(commits)`
- `rewrite_commit_description(revision, message)`

这样调用方只描述“我要什么仓库语义”，不需要再知道内部是历史遍历、快照、revset 还是 shell 命令。

备选方案：
- 保留现有 `RepoGateway` 形状，只把内部换成 actor：改动小，但低层细节仍继续泄漏到 app 层。
- 直接把更高层的业务对象如“record edit”“sync batch”也塞进仓库接口：会让仓库层混入 workflow 语义，边界过厚。

### 2. live 实现采用工作区级唯一 actor，而不是全局服务

每次 CLI 命令执行时，应用层会请求目标工作区对应的 `JjRepoActorHandle`。同一 `aclog` 进程内，如果这个工作区已经存在 live actor，就直接复用；如果不存在，才启动新的 actor。actor 持有当前工作区的 `workspace_root`，并把该工作区的 live 仓库请求排队串行处理。

这样既能保证顺序语义，又能避免引入长期存活的服务进程、跨命令状态和全局注册复杂度。对当前“单命令、单工作区”的程序模型来说，这是最贴合的作用域，同时也避免了同一进程里出现多个互不知晓的同工作区 actor。

备选方案：
- 全局 actor / registry：更接近通用运行时，但对当前 CLI 模型过重。
- 完全不做 actor，只靠调用链自然串行：当前可用，但无法把顺序保证固化为仓库边界能力。
 - 每次 `LiveDeps::new(...)` 都盲目新建 actor：会让同一进程内同工作区出现多个 live handle，削弱“唯一仓库入口”的约束。

### 3. actor 使用原生 Tokio channel，自行实现轻量消息循环

实现使用 `tokio::sync::mpsc` 接收请求，使用 `tokio::sync::oneshot` 回传结果，请求消息枚举按操作维度建模并携带 typed payload。这样可以最小化依赖，复用现有 Tokio runtime，也避免引入额外 actor 框架心智负担。

之所以不引入 `actix`、`ractor` 等第三方 actor 库，是因为当前需求只有：

- 单 mailbox
- 串行执行
- typed request / response
- 命令结束后自然退出

这些能力原生 Tokio 已足够覆盖，引入更成熟的 actor runtime 只会额外增加 supervision、registry、system lifecycle 等当前不需要的复杂度。

备选方案：
- `actix`：生态成熟，但 runtime 模型偏重，超出当前需求。
- `ractor` 等 actor 框架：功能完整，但对单 CLI 内部串行器来说过度设计。

### 4. actor 每次请求重新加载仓库状态，不跨写操作缓存 repo 视图

actor 的职责是串行化和封装，而不是做激进缓存。对于每个请求，live 实现继续复用 `src/vcs/read.rs` / `src/vcs/write.rs` 中的核心逻辑，但在请求内部重新加载仓库状态，而不是让 actor 长期持有可变的 repo / workspace 视图。

这样可以避免：

- 写后继续使用陈旧的 `ReadonlyRepo`
- 工作区快照与实际文件状态漂移
- 需要自己维护复杂的缓存失效策略

代价是多一些仓库加载开销，但当前数据规模和 CLI 交互频率足以接受，优先级应放在正确性。

备选方案：
- actor 长期缓存 repo / workspace：理论上可减少重复加载，但 stale state 风险高。
- 对读结果做二级缓存：当前收益不足，且会削弱“写后读立即可见”的保证。

### 5. 保持仓库 actor 与 ProblemProvider / OutputSink 解耦

`ProblemProvider` 和 `OutputSink` 继续作为独立依赖存在，只有仓库访问走 actor。这能保持边界清晰：

- actor 只负责本地 `jj` 仓库相关读写与顺序控制
- API 拉取、缓存读取和终端输出仍由各自依赖承担

这样测试替身也更容易维护，因为 fake 仓库、fake problem provider、fake output sink 不会被迫绑定成一个更重的运行时对象。

备选方案：
- 把所有 deps 都并进一个大 actor：接口会过于臃肿，也会把与 `jj` 无关的异步调用强行串行化。

### 6. 迁移策略分三步走：先引入新边界，再迁 app，最后收缩旧接口

迁移顺序固定为：

1. 定义新的语义化仓库 trait 与 actor-backed live 实现。
2. 让 `LiveDeps` 持有按工作区复用的 actor handle，并把 `src/app/mod.rs` 的顶层入口改为每次命令显式构造 live deps。
3. 迁移 `app/support` 和各 workflow 到新仓库接口，再删除或收缩旧的低层 `RepoGateway` / `vcs` 导出。

这样可以控制改动面，并在每一步维持测试可运行。

## Risks / Trade-offs

- [actor 引入后接口重构面较大] -> 先固定最小语义接口，再逐步迁移 app 层，避免一次性把 workflow 语义也塞进仓库层。
- [写后读一致性若处理不好会引入隐蔽 stale-state bug] -> actor 内部对每次请求重新加载仓库状态，不跨写缓存 repo 视图。
- [测试替身随着接口重构需要大面积更新] -> 保持 fake 仓库仍是普通 trait impl，不要求测试运行 live actor。
- [Tokio channel 方案增加少量异步样板代码] -> 用 typed request/response 和小而清晰的消息枚举控制复杂度，不额外引入 actor 框架。
- [旧 `vcs::*` 直接调用路径可能继续残留] -> 在 tasks 中明确包含导出收缩与直接调用清理，避免新旧边界并存太久。

## Migration Plan

1. 在 `src/vcs/` 下增加 actor 模块，补上 Tokio `sync` feature，并定义新的语义化仓库 trait。
2. 实现 actor-backed live 仓库 handle，把现有读写逻辑收进 actor 内部分发。
3. 重构 `LiveDeps` 与顶层命令入口，使每次命令创建一个 actor-backed live deps。
4. 迁移 `app/support`、`sync`、`record`、浏览和统计 workflow 到新仓库接口。
5. 更新 fake deps、真实 `jj` 集成测试与 actor 专项测试。
6. 收缩旧低层仓库接口和不再需要的直接导出。

回滚策略：

- 如果 actor 重构中途发现边界设计不理想，可以保留新 trait 定义，回退 live 实现到直接调用旧 `vcs::*` 的非 actor 版本。
- 如果某一步导致行为回归，优先回滚 app 层迁移，保持原有 workflow 与旧仓库接口继续工作。

## Open Questions

- 无。当前 change 固定采用自实现的轻量级 Tokio actor，不引入第三方 actor 框架。
