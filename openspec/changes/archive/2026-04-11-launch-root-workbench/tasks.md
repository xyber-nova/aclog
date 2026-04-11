## 1. CLI 与首页 workflow 骨架

- [x] 1.1 调整 `src/cli.rs` 顶层解析，让 `command` 变为可选并在无参时分发到新的首页 workflow，同时保持 `--help` 和既有子命令语义不变
- [x] 1.2 在 `src/app/` 新增首页 workflow 与本地摘要 view model，读取 `RecordIndex` 和 `.aclog/sync-session.toml`，但不依赖远端配置或网络
- [x] 1.3 扩展 `UserInterface`、`TerminalUi` 和 `FakeUi`，增加首页动作枚举与首页入口方法，保持“app 编排、UI 回传动作”的边界

## 2. 终端首页与导航集成

- [x] 2.1 在 `src/ui/terminal/` 新增首页页面，实现统一的“上下文 + 入口列表 + 摘要/详情 + 操作提示”布局以及 `Enter` / `Esc` / `q` / `?` / `j/k` 键位
- [x] 2.2 在首页中实现可恢复 sync 状态展示、本地训练摘要展示，以及进入 `sync`、`stats`、文件浏览、题目浏览的导航动作
- [x] 2.3 为首页新增 `record list` 只读快照查看页，复用现有 `record_list` 纯渲染输出并支持返回首页
- [x] 2.4 让首页按“退出首页 -> 运行子 workflow -> 返回首页”的方式集成现有 `sync`、`stats`、`record browse` 与列表查看，而不改变这些 workflow 的既有语义

## 3. 测试与回归

- [x] 3.1 更新 CLI 解析与 smoke tests，覆盖 `aclog` 无参进入首页、`aclog --help` 仍输出帮助，以及现有子命令继续可用
- [x] 3.2 增加首页 workflow 测试，覆盖空历史摘要、存在 sync-session 的恢复入口、以及从首页分发到 `sync` / `stats` / `browse` / `record list` 的动作
- [x] 3.3 为首页终端辅助渲染与动作映射补充单元测试，覆盖空状态、恢复提示、帮助切换和入口详情文案
