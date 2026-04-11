//! 真实终端 UI 的统一入口。
//!
//! 这一层负责：
//! - 管理 ratatui/crossterm 的终端生命周期
//! - 把应用层调用分发到具体页面模块
//! - 维护终端实现内部可复用的入口函数
//!
//! 这一层不负责：
//! - 决定业务流程
//! - 访问 API / `jj`
//! - 改写领域对象

mod browser;
mod common;
mod home;
mod selector;
mod stats;
mod sync;
pub(crate) mod theme;

use std::io::{self, Stdout};

use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    domain::{
        browser::BrowserQuery,
        problem::ProblemMetadata,
        record::{HistoricalSolveRecord, SyncSelection},
        record_index::RecordIndex,
        stats::{StatsDashboard, StatsSummary},
        submission::SubmissionRecord,
        sync_batch::{SyncBatchSession, SyncSessionChoice, SyncSessionItem},
    },
    ui::interaction::{HomeAction, HomeSummary, SyncBatchDetailAction, SyncBatchReviewAction},
};

/// 真实终端句柄类型别名，供各页面模块共享。
pub(crate) type TerminalHandle = Terminal<CrosstermBackend<Stdout>>;

/// 打开 submission 选择器，返回 sync workflow 需要的决策结果。
pub fn open_home(workspace_root: &std::path::Path, summary: &HomeSummary) -> Result<HomeAction> {
    run_in_terminal(|terminal| home::run_home_app(terminal, workspace_root, summary))
}

/// 打开 submission 选择器，返回 sync workflow 需要的决策结果。
pub fn select_submission(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncSelection> {
    run_in_terminal(|terminal| {
        selector::run_submission_app(
            terminal,
            problem_id,
            metadata,
            submissions,
            selector::SubmissionSelectorMode::Sync,
        )
    })
}

/// 为 `record bind/rebind` 打开 submission 选择器。
///
/// 这里对外只暴露“选中的 submission 或取消”，
/// 不把 sync 专用的 `Chore/Skip/Delete` 决策泄漏给 record workflow。
pub fn select_record_submission(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<Option<SubmissionRecord>> {
    let selection = run_in_terminal(|terminal| {
        selector::run_submission_app(
            terminal,
            problem_id,
            metadata,
            submissions,
            selector::SubmissionSelectorMode::Record,
        )
    })?;
    match selection {
        SyncSelection::Submission(record) => Ok(Some(record)),
        SyncSelection::Skip | SyncSelection::Chore | SyncSelection::Delete => Ok(None),
    }
}

/// 为 `record rebind` 打开历史记录选择器。
pub fn select_record_to_rebind(
    problem_id: &str,
    file_name: &str,
    records: &[HistoricalSolveRecord],
) -> Result<Option<HistoricalSolveRecord>> {
    run_in_terminal(|terminal| selector::run_record_app(terminal, problem_id, file_name, records))
}

/// 为已删除文件提供显式确认界面。
pub fn confirm_deleted_file(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
) -> Result<SyncSelection> {
    run_in_terminal(|terminal| selector::run_delete_app(terminal, problem_id, metadata))
}

/// 在检测到恢复会话时，让用户决定恢复或重建。
pub fn choose_sync_session_action(
    workspace_root: &std::path::Path,
    session: &SyncBatchSession,
) -> Result<SyncSessionChoice> {
    run_in_terminal(|terminal| sync::run_sync_session_choice_app(terminal, workspace_root, session))
}

/// 打开 sync 预览页，并返回预览层面的动作语义。
pub fn review_sync_batch_action(
    workspace_root: &std::path::Path,
    session: &SyncBatchSession,
) -> Result<SyncBatchReviewAction> {
    run_in_terminal(|terminal| sync::run_sync_batch_review_app(terminal, workspace_root, session))
}

/// 兼容旧接口的 sync 预览入口。
///
/// 新预览页允许直接做安全快操，因此这里需要把预览决策临时排队，
/// 等 app workflow 真正处理到该项时再统一消费。
pub fn review_sync_batch(
    workspace_root: &std::path::Path,
    session: &SyncBatchSession,
) -> Result<Option<usize>> {
    match review_sync_batch_action(workspace_root, session)? {
        SyncBatchReviewAction::Pause => Ok(None),
        SyncBatchReviewAction::Open(index) => Ok(Some(index)),
        SyncBatchReviewAction::Decide { index, selection } => {
            sync::queue_preview_action(session, index, selection);
            Ok(Some(index))
        }
    }
}

/// 打开单个 sync 批次项的详情页，并返回“返回上一层 / 明确决策”。
pub fn select_sync_batch_detail_action(
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncBatchDetailAction> {
    run_in_terminal(|terminal| sync::run_sync_item_app(terminal, item, metadata, submissions))
}

/// 兼容旧接口的 sync 详情入口。
///
/// 优先消费预览页已经排队的快操；如果没有排队动作，再真正打开详情页。
pub fn select_sync_batch_action(
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncSelection> {
    if let Some(selection) = sync::take_queued_preview_action(item) {
        return Ok(selection);
    }
    match select_sync_batch_detail_action(item, metadata, submissions)? {
        SyncBatchDetailAction::Back => Ok(SyncSelection::Skip),
        SyncBatchDetailAction::Decide(selection) => Ok(selection),
        SyncBatchDetailAction::Quit => Ok(SyncSelection::Skip),
    }
}

/// 打开记录浏览工作台。
pub fn open_record_browser(
    workspace_root: &std::path::Path,
    query: &BrowserQuery,
    index: &RecordIndex,
) -> Result<()> {
    run_in_terminal(|terminal| browser::run_browser_app(terminal, workspace_root, query, index))
}

/// 打开带 review 建议的统计工作台。
pub fn show_stats_dashboard(
    workspace_root: &std::path::Path,
    dashboard: &StatsDashboard,
    index: &RecordIndex,
) -> Result<()> {
    run_in_terminal(|terminal| {
        stats::run_stats_dashboard_app(terminal, workspace_root, dashboard, index)
    })
}

/// 打开只含统计概览的工作台。
pub fn show_stats(workspace_root: &std::path::Path, summary: &StatsSummary) -> Result<()> {
    run_in_terminal(|terminal| stats::run_stats_app(terminal, workspace_root, summary))
}

/// 统一管理真实终端生命周期。
///
/// 页面模块只需要关心“如何在现有 terminal 上运行交互循环”，
/// 不需要重复处理 raw mode、alternate screen 和光标恢复。
fn run_in_terminal<T>(run: impl FnOnce(&mut TerminalHandle) -> Result<T>) -> Result<T> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
