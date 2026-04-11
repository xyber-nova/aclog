//! sync workflow 的终端页面。
//!
//! 这里负责：
//! - 恢复页
//! - 批次预览页
//! - 单项详情页
//! - 预览页快操与详情页决策之间的交接
//!
//! 这里不负责：
//! - 生成批次
//! - 落库或创建 commit
//! - 解释 warning 的业务来源

use std::sync::{Mutex, OnceLock};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Constraint,
    style::Modifier,
    text::{Line, Span},
    widgets::{Cell, Row, Table, TableState},
};

use crate::{
    domain::{
        problem::ProblemMetadata,
        record::SyncSelection,
        submission::SubmissionRecord,
        sync_batch::{
            SyncBatchSession, SyncChangeKind, SyncItemStatus, SyncSessionChoice, SyncSessionItem,
        },
    },
    problem::{human_problem_id, provider_label},
    ui::{
        interaction::{SyncBatchDetailAction, SyncBatchReviewAction},
        terminal::{
            TerminalHandle,
            common::{
                clamp_selection, footer_panel, initial_selection_for_count, is_help, lines_panel,
                move_selection, root_vertical_layout, split_main_with_summary, text_panel,
            },
            theme,
        },
    },
    utils::normalize_verdict,
};

#[derive(Debug, Clone)]
struct QueuedSyncAction {
    /// 用文件名 + change kind 做最小交接键，避免引入额外 session 状态。
    file: String,
    kind: SyncChangeKind,
    selection: SyncSelection,
}

/// 预览快操的单槽暂存区。
///
/// 预览页返回后，app workflow 仍会按照旧节奏进入“处理当前项”的步骤，
/// 因此这里需要一个极小的桥接状态，把快操结果交给下一步消费。
fn queued_action_slot() -> &'static Mutex<Option<QueuedSyncAction>> {
    static SLOT: OnceLock<Mutex<Option<QueuedSyncAction>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// 记录预览页上已经做出的快操决策。
pub(crate) fn queue_preview_action(
    session: &SyncBatchSession,
    index: usize,
    selection: SyncSelection,
) {
    let Some(item) = session.items.get(index) else {
        return;
    };
    *queued_action_slot().lock().unwrap() = Some(QueuedSyncAction {
        file: item.file.clone(),
        kind: item.kind,
        selection,
    });
}

/// 如果当前详情项正好对应预览页快操，就直接消费该决策。
pub(crate) fn take_queued_preview_action(item: &SyncSessionItem) -> Option<SyncSelection> {
    let mut guard = queued_action_slot().lock().unwrap();
    let Some(queued) = guard.as_ref() else {
        return None;
    };
    // 预览页的快操是在 app workflow 下一次处理该项时真正落库的，
    // 所以这里用 file + change kind 做一次轻量交接。
    if queued.file != item.file || queued.kind != item.kind {
        return None;
    }
    guard.take().map(|entry| entry.selection)
}

/// 恢复会话选择页。
pub(crate) fn run_sync_session_choice_app(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    session: &SyncBatchSession,
) -> Result<SyncSessionChoice> {
    let mut help_visible = false;
    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 4);
            frame.render_widget(
                lines_panel(
                    "恢复批次",
                    vec![
                        Line::from(format!("工作区: {}", workspace_root.display())),
                        Line::from(format!(
                            "检测到未完成 sync 批次，待处理项 {} 个",
                            session
                                .items
                                .iter()
                                .filter(|item| item.status == SyncItemStatus::Pending)
                                .count()
                        )),
                    ],
                ),
                header,
            );
            frame.render_widget(
                text_panel(
                    "恢复操作",
                    "r 继续恢复当前批次\nn 丢弃旧批次并按当前工作区重建\nEsc 或 q 退出",
                ),
                content,
            );
            frame.render_widget(
                footer_panel(
                    "r 恢复  n 重建  Esc/q 退出",
                    &[
                        "r: 继续处理当前批次",
                        "n: 丢弃旧批次并重建",
                        "Esc/q: 退出当前工作台",
                    ],
                    help_visible,
                ),
                footer,
            );
        })?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if is_help(key.code) {
            help_visible = !help_visible;
            continue;
        }
        match key.code {
            KeyCode::Char('r') => return Ok(SyncSessionChoice::Resume),
            KeyCode::Char('n') => return Ok(SyncSessionChoice::Rebuild),
            KeyCode::Esc | KeyCode::Char('q') => return Ok(SyncSessionChoice::Quit),
            _ => {}
        }
    }
}

/// sync 批次预览页。
///
/// 这是整个 sync workflow 的主工作台：
/// - 左侧浏览批次项
/// - 右侧看摘要、告警和默认候选
/// - 对安全动作做预览层快操
pub(crate) fn run_sync_batch_review_app(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    session: &SyncBatchSession,
) -> Result<SyncBatchReviewAction> {
    let mut help_visible = false;
    let display_indices = preview_item_indices(session);
    let mut state =
        TableState::default().with_selected(initial_selection_for_count(display_indices.len()));

    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 4);
            let [list_area, detail_area] = split_main_with_summary(content, 60, 40);

            frame.render_widget(
                lines_panel(
                    "Sync 批次预览",
                    vec![
                        Line::from(format!("工作区: {}", workspace_root.display())),
                        Line::from("预览页可浏览批次、执行安全快操，或进入详情页选择 submission"),
                    ],
                ),
                header,
            );

            let header_row = Row::new([
                Cell::from("文件"),
                Cell::from("题号"),
                Cell::from("来源"),
                Cell::from("类型"),
                Cell::from("状态"),
                Cell::from("提交数"),
                Cell::from("默认候选"),
            ])
            .style(theme::accent_style().add_modifier(Modifier::BOLD));
            let rows = display_indices
                .iter()
                .filter_map(|index| session.items.get(*index))
                .map(|item| {
                    Row::new([
                        Cell::from(item.file.clone()),
                        Cell::from(
                            item.problem_id
                                .as_deref()
                                .map(human_problem_id)
                                .unwrap_or_else(|| "-".to_string()),
                        ),
                        Cell::from(provider_label(item.provider)),
                        Cell::from(change_kind_label(item.kind))
                            .style(theme::change_kind_style(item.kind)),
                        Cell::from(sync_status_label(item.status))
                            .style(theme::sync_status_style(item.status)),
                        Cell::from(
                            item.submissions
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                        ),
                        Cell::from(
                            item.default_submission_id
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                        ),
                    ])
                })
                .collect::<Vec<_>>();
            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(28),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(12),
                    Constraint::Length(10),
                    Constraint::Length(12),
                ],
            )
            .header(header_row)
            .block(crate::ui::terminal::common::panel("批次列表"))
            .row_highlight_style(theme::selected_row_style())
            .highlight_symbol(theme::FOCUS_SYMBOL);
            frame.render_stateful_widget(table, list_area, &mut state);

            let summary = state
                .selected()
                .and_then(|index| display_indices.get(index))
                .and_then(|index| session.items.get(*index))
                .map(render_sync_item_summary)
                .unwrap_or_else(|| "当前没有可处理项".to_string());
            frame.render_widget(text_panel("当前项摘要", summary), detail_area);

            frame.render_widget(
                footer_panel(
                    "j/k/↑/↓ 移动  Enter 进入详情  c 标记 chore  s 跳过  d 删除  Esc/q 暂停",
                    &[
                        "Enter: 打开当前项详情页",
                        "c: 对 active 项直接标记为 chore",
                        "s: 直接跳过当前项",
                        "d: 对 deleted 项直接确认 remove",
                        "Esc/q: 暂停并保留当前批次",
                    ],
                    help_visible,
                ),
                footer,
            );
        })?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if is_help(key.code) {
            help_visible = !help_visible;
            continue;
        }
        if let Some(next) = move_selection(key.code, state.selected(), display_indices.len()) {
            state.select(next);
            continue;
        }

        let selected = state.selected().unwrap_or(0);
        let Some(item_index) = display_indices.get(selected).copied() else {
            continue;
        };
        let Some(item) = session.items.get(item_index) else {
            continue;
        };
        match key.code {
            KeyCode::Enter if item.status == SyncItemStatus::Pending => {
                return Ok(SyncBatchReviewAction::Open(item_index));
            }
            KeyCode::Char('c')
                if item.status == SyncItemStatus::Pending
                    && matches!(item.kind, SyncChangeKind::Active) =>
            {
                return Ok(SyncBatchReviewAction::Decide {
                    index: item_index,
                    selection: SyncSelection::Chore,
                });
            }
            KeyCode::Char('s') if item.status == SyncItemStatus::Pending => {
                return Ok(SyncBatchReviewAction::Decide {
                    index: item_index,
                    selection: SyncSelection::Skip,
                });
            }
            KeyCode::Char('d')
                if item.status == SyncItemStatus::Pending
                    && matches!(item.kind, SyncChangeKind::Deleted) =>
            {
                // 预览页只开放删除项的直接 remove；
                // active 文件仍然需要进详情页完成绑定，避免把复杂选择塞进列表页。
                return Ok(SyncBatchReviewAction::Decide {
                    index: item_index,
                    selection: SyncSelection::Delete,
                });
            }
            KeyCode::Esc | KeyCode::Char('q') => return Ok(SyncBatchReviewAction::Pause),
            _ => {}
        }
    }
}

/// 单个 sync 批次项的详情页入口。
///
/// 根据变更类型路由到不同的页面，但对上层保持统一返回语义。
pub(crate) fn run_sync_item_app(
    terminal: &mut TerminalHandle,
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncBatchDetailAction> {
    match item.kind {
        SyncChangeKind::Deleted => run_sync_delete_app(terminal, item, metadata),
        // active 变更继续走详情页，方便同时查看告警、上下文和精确 submission。
        SyncChangeKind::Active => run_sync_submission_app(terminal, item, metadata, submissions),
    }
}

/// 删除项详情页。
fn run_sync_delete_app(
    terminal: &mut TerminalHandle,
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
) -> Result<SyncBatchDetailAction> {
    let mut help_visible = false;
    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 5);
            frame.render_widget(
                lines_panel("确认删除", sync_item_header_lines(item, metadata)),
                header,
            );
            frame.render_widget(
                text_panel(
                    "删除动作",
                    "检测到题解文件已删除\nEnter 记为 remove\ns 跳过当前项\nEsc 返回批次预览",
                ),
                content,
            );
            frame.render_widget(
                footer_panel(
                    "Enter 确认删除  s 跳过  Esc 返回  q 退出",
                    &[
                        "Enter: 将当前项记为 remove",
                        "s: 跳过当前项",
                        "Esc: 返回批次预览",
                        "q: 直接退出当前工作台",
                    ],
                    help_visible,
                ),
                footer,
            );
        })?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if is_help(key.code) {
            help_visible = !help_visible;
            continue;
        }
        match key.code {
            KeyCode::Enter => return Ok(SyncBatchDetailAction::Decide(SyncSelection::Delete)),
            KeyCode::Char('s') => return Ok(SyncBatchDetailAction::Decide(SyncSelection::Skip)),
            KeyCode::Esc => return Ok(SyncBatchDetailAction::Back),
            KeyCode::Char('q') => return Ok(SyncBatchDetailAction::Quit),
            _ => {}
        }
    }
}

/// 活跃文件详情页，负责 submission 精确选择。
fn run_sync_submission_app(
    terminal: &mut TerminalHandle,
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncBatchDetailAction> {
    let mut help_visible = false;
    let mut state =
        TableState::default().with_selected(initial_selection_for_count(submissions.len()));

    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 6);
            let [list_area, detail_area] = split_main_with_summary(content, 60, 40);
            frame.render_widget(
                lines_panel("选择同步结果", sync_item_header_lines(item, metadata)),
                header,
            );

            if submissions.is_empty() {
                frame.render_widget(text_panel("候选列表", "未找到提交记录"), list_area);
                frame.render_widget(
                    text_panel(
                        "当前项摘要",
                        format!(
                            "{}\n\n可执行动作:\n- c 标记 chore\n- s 跳过当前项\n- Esc 返回批次预览",
                            render_sync_item_summary(item)
                        ),
                    ),
                    detail_area,
                );
            } else {
                let header_row = Row::new([
                    Cell::from("提交时间"),
                    Cell::from("用户"),
                    Cell::from("ID"),
                    Cell::from("结果"),
                    Cell::from("分数"),
                    Cell::from("耗时"),
                    Cell::from("内存"),
                ])
                .style(theme::accent_style().add_modifier(Modifier::BOLD));
                let rows = submissions
                    .iter()
                    .map(|record| {
                        Row::new([
                            Cell::from(
                                record
                                    .submitted_at
                                    .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or_else(|| "---- -- -- --:--".to_string()),
                            ),
                            Cell::from(record.submitter.clone()),
                            Cell::from(record.submission_id.to_string()),
                            Cell::from(normalize_verdict(&record.verdict).into_owned())
                                .style(theme::verdict_style(&record.verdict)),
                            Cell::from(
                                record
                                    .score
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "-".to_string()),
                            ),
                            Cell::from(
                                record
                                    .time_ms
                                    .map(|value| format!("{value}ms"))
                                    .unwrap_or_else(|| "-".to_string()),
                            ),
                            Cell::from(
                                record
                                    .memory_mb
                                    .map(|value| format!("{value:.1}MB"))
                                    .unwrap_or_else(|| "-".to_string()),
                            ),
                        ])
                    })
                    .collect::<Vec<_>>();
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(16),
                        Constraint::Length(14),
                        Constraint::Length(9),
                        Constraint::Length(8),
                        Constraint::Length(6),
                        Constraint::Length(8),
                        Constraint::Length(8),
                    ],
                )
                .header(header_row)
                .block(crate::ui::terminal::common::panel("提交记录"))
                .row_highlight_style(theme::selected_row_style())
                .highlight_symbol(theme::FOCUS_SYMBOL);
                frame.render_stateful_widget(table, list_area, &mut state);

                let detail = submissions
                    .get(clamp_selection(
                        state.selected().unwrap_or(0),
                        submissions.len(),
                    ))
                    .map(|record| render_submission_summary_lines(item, record))
                    .unwrap_or_else(|| {
                        render_sync_item_summary(item)
                            .lines()
                            .map(|line| Line::from(line.to_string()))
                            .collect::<Vec<_>>()
                    });
                frame.render_widget(lines_panel("当前项摘要", detail), detail_area);
            }

            frame.render_widget(
                footer_panel(
                    "j/k/↑/↓ 移动  Enter 选择 submission  c 标记 chore  s 跳过  Esc 返回",
                    &[
                        "Enter: 绑定当前 submission",
                        "c: 将当前项标记为 chore",
                        "s: 跳过当前项",
                        "Esc: 返回批次预览",
                        "q: 直接退出当前工作台",
                    ],
                    help_visible,
                ),
                footer,
            );
        })?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if is_help(key.code) {
            help_visible = !help_visible;
            continue;
        }
        if let Some(next) = move_selection(key.code, state.selected(), submissions.len()) {
            state.select(next);
            continue;
        }

        match key.code {
            KeyCode::Enter => {
                if let Some(record) = state
                    .selected()
                    .and_then(|index| submissions.get(index).cloned())
                {
                    return Ok(SyncBatchDetailAction::Decide(SyncSelection::Submission(
                        record,
                    )));
                }
            }
            KeyCode::Char('c') => return Ok(SyncBatchDetailAction::Decide(SyncSelection::Chore)),
            KeyCode::Char('s') => return Ok(SyncBatchDetailAction::Decide(SyncSelection::Skip)),
            KeyCode::Esc => return Ok(SyncBatchDetailAction::Back),
            KeyCode::Char('q') => return Ok(SyncBatchDetailAction::Quit),
            _ => {}
        }
    }
}

fn render_submission_summary_lines(
    item: &SyncSessionItem,
    record: &SubmissionRecord,
) -> Vec<Line<'static>> {
    let mut lines = render_sync_item_summary(item)
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect::<Vec<_>>();
    let verdict = normalize_verdict(&record.verdict).into_owned();
    lines.push(Line::from(""));
    lines.push(Line::from("当前 submission:"));
    lines.push(Line::from(vec![
        Span::raw("结果: "),
        Span::styled(verdict.clone(), theme::verdict_style(&verdict)),
    ]));
    lines.push(Line::from(format!("提交 ID: {}", record.submission_id)));
    lines.push(Line::from(format!("用户: {}", record.submitter)));
    lines.push(Line::from(format!(
        "提交时间: {}",
        record
            .submitted_at
            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string()),
    )));
    lines
}

/// 单项详情页头部摘要。
fn sync_item_header_lines(
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("文件: {}", item.file)),
        Line::from(format!(
            "题号: {}",
            item.problem_id
                .as_deref()
                .map(human_problem_id)
                .unwrap_or_else(|| "-".to_string())
        )),
        Line::from(format!("来源: {}", provider_label(item.provider))),
        Line::from(format!(
            "类型: {}  状态: {}",
            change_kind_label(item.kind),
            sync_status_label(item.status)
        )),
    ];
    if let Some(contest) = metadata
        .and_then(|item| item.contest.as_deref())
        .or(item.contest.as_deref())
    {
        lines.push(Line::from(format!("比赛: {contest}")));
    }
    if let Some(metadata) = metadata {
        lines.push(Line::from(format!(
            "标题: {}  难度: {}",
            metadata.title,
            metadata.difficulty.as_deref().unwrap_or("-")
        )));
    }
    lines
}

/// 右侧摘要区的统一文案生成。
///
/// 这里集中输出文件、题号、状态、默认候选和告警，
/// 避免预览页和详情页各写一套不一致的摘要。
fn render_sync_item_summary(item: &SyncSessionItem) -> String {
    let mut lines = vec![
        format!("文件: {}", item.file),
        format!(
            "题号: {}",
            item.problem_id
                .as_deref()
                .map(human_problem_id)
                .unwrap_or_else(|| "-".to_string())
        ),
        format!("来源: {}", provider_label(item.provider)),
    ];
    if let Some(contest) = item.contest.as_deref() {
        lines.push(format!("比赛: {contest}"));
    }
    lines.extend([
        format!("类型: {}", change_kind_label(item.kind)),
        format!("状态: {}", sync_status_label(item.status)),
        format!(
            "提交记录数: {}",
            item.submissions
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "默认候选: {}",
            item.default_submission_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
    ]);
    if let Some(reason) = item.invalid_reason.as_deref() {
        lines.push(format!("{}状态说明: {reason}", theme::WARNING_SYMBOL));
    }
    if !item.warnings.is_empty() {
        lines.push(String::new());
        lines.push("告警".to_string());
        lines.extend(
            item.warnings
                .iter()
                .map(|warning| format!("{}{}", theme::WARNING_SYMBOL, warning.message)),
        );
    }
    lines.join("\n")
}

fn preview_item_indices(session: &SyncBatchSession) -> Vec<usize> {
    let mut pending = Vec::new();
    let mut deferred = Vec::new();
    for (index, item) in session.items.iter().enumerate() {
        if item.status == SyncItemStatus::Pending {
            pending.push(index);
        } else {
            deferred.push(index);
        }
    }
    pending.extend(deferred);
    pending
}

/// sync 变更类型的人类可读标签。
fn change_kind_label(kind: SyncChangeKind) -> &'static str {
    match kind {
        SyncChangeKind::Active => "已修改",
        SyncChangeKind::Deleted => "已删除",
    }
}

/// sync 状态的人类可读标签。
fn sync_status_label(status: SyncItemStatus) -> &'static str {
    match status {
        SyncItemStatus::Pending => "待处理",
        SyncItemStatus::Planned => "已决待提交",
        SyncItemStatus::Skipped => "已跳过",
        SyncItemStatus::Committed => "已提交",
        SyncItemStatus::Invalid => "已失效",
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use crossterm::event::KeyCode;

    use super::{
        change_kind_label, preview_item_indices, render_sync_item_summary, sync_status_label,
    };
    use crate::domain::sync_batch::{
        SyncBatchSession, SyncChangeKind, SyncItemStatus, SyncSessionItem, SyncWarning,
        SyncWarningCode,
    };

    #[test]
    fn summary_mentions_warning_and_default_submission() {
        let summary = render_sync_item_summary(&SyncSessionItem {
            file: "P1001.cpp".to_string(),
            problem_id: Some("luogu:P1001".to_string()),
            provider: crate::problem::ProblemProvider::Luogu,
            contest: None,
            kind: SyncChangeKind::Active,
            status: SyncItemStatus::Pending,
            submissions: Some(3),
            default_submission_id: Some(42),
            decision: None,
            warnings: vec![SyncWarning {
                code: SyncWarningCode::DuplicateSubmission,
                message: "重复绑定".to_string(),
            }],
            invalid_reason: None,
        });

        assert!(summary.contains("默认候选: 42"));
        assert!(summary.contains("重复绑定"));
    }

    #[test]
    fn labels_match_current_sync_semantics() {
        assert_eq!(change_kind_label(SyncChangeKind::Deleted), "已删除");
        assert_eq!(sync_status_label(SyncItemStatus::Invalid), "已失效");
        assert!(matches!(KeyCode::Char('s'), KeyCode::Char('s')));
    }

    #[test]
    fn preview_lists_pending_items_before_decided_items() {
        let session = SyncBatchSession {
            created_at: chrono::FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                .single()
                .unwrap(),
            items: vec![
                SyncSessionItem {
                    file: "P1001.cpp".to_string(),
                    problem_id: Some("luogu:P1001".to_string()),
                    provider: crate::problem::ProblemProvider::Luogu,
                    contest: None,
                    kind: SyncChangeKind::Active,
                    status: SyncItemStatus::Planned,
                    submissions: None,
                    default_submission_id: None,
                    decision: None,
                    warnings: Vec::new(),
                    invalid_reason: None,
                },
                SyncSessionItem {
                    file: "P1002.cpp".to_string(),
                    problem_id: Some("luogu:P1002".to_string()),
                    provider: crate::problem::ProblemProvider::Luogu,
                    contest: None,
                    kind: SyncChangeKind::Active,
                    status: SyncItemStatus::Pending,
                    submissions: None,
                    default_submission_id: None,
                    decision: None,
                    warnings: Vec::new(),
                    invalid_reason: None,
                },
                SyncSessionItem {
                    file: "P1003.cpp".to_string(),
                    problem_id: Some("luogu:P1003".to_string()),
                    provider: crate::problem::ProblemProvider::Luogu,
                    contest: None,
                    kind: SyncChangeKind::Active,
                    status: SyncItemStatus::Skipped,
                    submissions: None,
                    default_submission_id: None,
                    decision: None,
                    warnings: Vec::new(),
                    invalid_reason: None,
                },
                SyncSessionItem {
                    file: "P1004.cpp".to_string(),
                    problem_id: Some("luogu:P1004".to_string()),
                    provider: crate::problem::ProblemProvider::Luogu,
                    contest: None,
                    kind: SyncChangeKind::Active,
                    status: SyncItemStatus::Pending,
                    submissions: None,
                    default_submission_id: None,
                    decision: None,
                    warnings: Vec::new(),
                    invalid_reason: None,
                },
            ],
        };

        assert_eq!(preview_item_indices(&session), vec![1, 3, 0, 2]);
    }
}
