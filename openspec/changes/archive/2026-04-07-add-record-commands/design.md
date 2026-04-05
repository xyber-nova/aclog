## Context

当前 `aclog` 已经有一条稳定的 `sync` 主流程：从工作区变更里发现题目文件、拉洛谷题目与提交记录、通过 TUI 做显式选择、最后生成 `solve(...)` / `chore(...)` / `remove(...)` commit。它适合“文件刚改完，顺手记录”这类场景，但不覆盖“这份已有解法漏记了”“这条历史记录绑错了 submission”“我想快速看哪些文件已经记过”这些文件级维护动作。

这次改动同时涉及 CLI 命令面、`jj` 历史读取、记录索引、rewrite 流程和交互边界，适合先把模型固定下来。最关键的约束是：`record` 的主对象是具体解法文件，不是抽象题目；`aclog` 本身是 CLI 工具，TUI 只是其中的交互模式，不能成为功能的唯一依赖。

## Goals / Non-Goals

**Goals:**
- 提供 `aclog record bind <file>`、`aclog record rebind <file>` 和 `aclog record list` 三个文件级命令。
- 把“解法文件 + 题号 + submission 绑定”建模为一条标准 `solve(...)` 记录，并复用现有 commit message 结构。
- 让 `rebind` 通过 `jj` rewrite 修正既有 `solve(...)` 记录，而不是追加新的 correction commit。
- 明确命令层与交互式终端界面的边界：命令层负责校验、数据拉取、`jj` 操作和流程编排；交互界面只负责从候选集合里选择。
- 明确所有选择步骤都可以由非交互 CLI 参数完全表达，保证未来或脚本场景下无需进入 TUI 也能完成同样的操作。

**Non-Goals:**
- 不新增按抽象题目直接操作的 `record` 入口；首版只支持按解法文件操作。
- 不支持对不存在或未被 `jj` 跟踪的文件执行 `bind`。
- 不引入新的 commit 类型；正式做题记录仍然只有 `solve(...)`。
- 不在首版处理文件重命名迁移、同一路径多语言文件归并、或批量重绑。

## Decisions

### 1. `record` 的主键是工作区内的解法文件路径

`bind`、`rebind` 和 `list` 都以具体解法文件为对象。系统会把工作区相对路径作为记录身份的一部分，并继续在 `solve(...)` commit message 中保留 `File:` 字段。题号仍然从文件名解析；如果文件名无法提取合法题号，命令直接失败，而不是再引入另一套手工题号绑定语义。

这样做的原因是用户真正要维护的是“这份 cpp 解法对应哪条 submission”，而不是抽象的题目对象。按文件建模后，同一道题的多份解法可以自然并存，`record list` 也能稳定按文件展示。

备选方案：
- 按 `problem-id` 操作：会把同题多份解法混在一起，`rebind` 无法精确表达“哪份代码绑错了”。
- 让 `bind` 同时接受任意 `--problem` 覆盖：会把本来明确的文件语义重新拉回题目语义，首版不需要。

### 2. `bind` 只允许为当前存在且被 `jj` 跟踪的文件补录

`record bind <file>` 首先检查：
- 文件路径存在
- 文件被当前 `jj` 工作区跟踪
- 文件名可提取题号

只有满足这些条件，命令才继续拉题目 metadata 和 submission 列表。这样能保证“补录的是当前工作区里这份真实存在的解法”，避免把 `record` 变成对任意路径字符串写历史的接口。

备选方案：
- 允许对不存在的路径补录：实现简单，但会制造无法落到工作区实体文件的历史记录。
- 允许未跟踪文件补录：会绕过 `jj` 工作流的一致性约束。

### 3. `rebind` 重写用户选中的既有 `solve(...)` 记录，而不是追加新记录

`record rebind <file>` 会先读取该文件关联的历史 `solve(...)` 记录列表，再让用户选择要重写的那一条。随后系统只从同题 submission 中选择新的绑定，并使用 `jj` rewrite 修改被选中的 commit message。重写后的记录仍然是普通 `solve(...)` message，不新增 correction / rebind 专用类型。

`problem-id` 和 `File:` 默认保持不变；改变的是 submission 相关字段，以及基于当前题目 metadata 重建的标题、标签、难度和来源等静态字段。

备选方案：
- 追加 correction commit：历史更保守，但会把“修正一次错误绑定”也变成一条额外训练记录，污染 `stats` 和 `record list` 语义。
- 只允许改最新一条：实现更简单，但不满足用户“手动挑一条出来重写”的明确要求。

### 4. 命令语义必须先独立成立，TUI 只是交互模式

所有 `record` 选择步骤都必须有非交互 CLI 等价输入：
- `bind` 支持通过 `--submission-id <id>` 直接指定 submission，提供后跳过 submission 选择 TUI。
- `rebind` 支持通过 `--record-rev <revset>` 指定要重写的历史记录，并通过 `--submission-id <id>` 指定新的 submission；当两者都提供时，命令必须能在不进入 TUI 的情况下完成重绑。
- 当 CLI 参数只补齐一部分选择时，只对剩余未决步骤进入 TUI。
- `record list` 始终是纯 CLI 文本输出，不进入 TUI。

这条决策的重点不是“默认不用 TUI”，而是“命令语义必须先独立于界面存在”。TUI 负责提高交互效率，非交互参数负责能力完备性和可脚本化。

这两类参数的语义固定如下：
- `--submission-id <id>`：直接指定目标 submission。系统必须校验该 submission 属于目标文件解析出的同一 `problem-id`；校验失败时直接报错，不回退到 TUI。
- `--record-rev <revset>`：仅用于 `rebind`，直接指定要重写的那条历史记录。系统必须要求该 revset 解析到唯一一条 commit，并校验它是 aclog 生成的标准 `solve(...)` 记录，且其 `File:` 与目标 `<file>` 匹配、题号也与该文件匹配；任一条件不满足时直接报错，不回退到 TUI。

因此首版组合规则为：
- `record bind <file>`：未提供 `--submission-id` 时进入 submission TUI。
- `record bind <file> --submission-id <id>`：全程不进入 TUI。
- `record rebind <file>`：先进入旧记录选择 TUI，再进入 submission TUI。
- `record rebind <file> --record-rev <revset>`：跳过旧记录选择 TUI，只进入 submission TUI。
- `record rebind <file> --submission-id <id>`：先进入旧记录选择 TUI，不进入 submission TUI。
- `record rebind <file> --record-rev <revset> --submission-id <id>`：全程不进入 TUI。

### 5. TUI 模块只提供纯选择器，不承载业务流程

`src/tui.rs` 只负责把 CLI 层已经准备好的候选数据渲染成选择界面，并返回用户选中的项。它不直接访问 API、不查询 `jj`、不决定命令流程。首版应只有两类可复用选择器：
- submission 选择器
- rebind 历史记录选择器

这样可以把“记录候选如何生成”“非交互参数是否已足够跳过交互”全部留在命令层，避免界面层反向拥有业务状态。

### 6. `record list` 以文件为行单位展示当前记录状态

`record list` 输出的是“每个已记录解法文件的当前状态”，而不是题目聚合视图。对同一路径出现多条 `solve(...)` 历史时，只展示最新一条；对同一道题的多个文件，则分别列出多行。输出优先使用 CLI 表格/文本，而不是进入 alternate screen。

这条规则让 `record list` 和 `record bind/rebind` 使用同一套文件级对象模型；题目级聚合仍然交给 `stats`。

## Risks / Trade-offs

- [文件路径成为记录身份的一部分] -> 重命名后的历史归属不会自动迁移；首版明确不处理 rename migration，后续如需支持再单独做 change。
- [`rebind` 依赖 `jj` rewrite] -> 需要小心限制只重写 aclog 生成的标准 `solve(...)` 记录，并在实现里补针对性测试。
- [非交互参数与 TUI 双模式会增加参数面] -> 用“CLI 参数补齐则跳过对应 TUI 步骤”的统一规则收敛，而不是再设计两套分叉流程。
- [`record list` 不做题目聚合] -> 题目级总览需要继续使用 `stats`；这是有意区分“文件记录视图”和“题目统计视图”。
