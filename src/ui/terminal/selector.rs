//! 轻量选择器页面。
//!
//! 这一模块承载几类“小而专”的交互：
//! - 选择 submission
//! - 选择要 rebind 的历史记录
//! - 确认删除
//!
//! 它们共享相同的列表 + 摘要结构，但不会承担完整工作台状态机。

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
        record::{HistoricalSolveRecord, SyncSelection},
        submission::SubmissionRecord,
    },
    ui::terminal::{
        TerminalHandle,
        common::{
            clamp_selection, footer_panel, initial_selection_for_count, is_help, lines_panel,
            move_selection, root_vertical_layout, split_main_with_summary, text_panel,
        },
        theme,
    },
    utils::normalize_verdict,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmissionSelectorMode {
    /// sync workflow 使用，允许 `chore/skip` 等额外动作。
    Sync,
    /// record workflow 使用，只允许“选中一条提交或取消”。
    Record,
}

/// submission 选择器主循环。
///
/// 职责边界：
/// - 展示候选 submission
/// - 返回用户显式决策
/// - 不负责推断默认提交，也不负责写入记录
pub(crate) fn run_submission_app(
    terminal: &mut TerminalHandle,
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
    mode: SubmissionSelectorMode,
) -> Result<SyncSelection> {
    let mut help_visible = false;
    let mut state =
        TableState::default().with_selected(initial_selection_for_count(submissions.len()));

    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 4);
            let [list_area, detail_area] = split_main_with_summary(content, 60, 40);

            frame.render_widget(
                lines_panel(
                    selector_title(mode),
                    problem_header_lines(problem_id, metadata),
                ),
                header,
            );

            if submissions.is_empty() {
                frame.render_widget(
                    text_panel("候选列表", submission_empty_state_text(mode)),
                    list_area,
                );
                frame.render_widget(
                    text_panel(
                        "当前摘要",
                        "当前没有可用 submission\n请使用帮助区中的动作完成后续处理",
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
                    .map(build_submission_row)
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
                .block(crate::ui::terminal::common::panel("候选列表"))
                .row_highlight_style(theme::selected_row_style())
                .highlight_symbol(theme::FOCUS_SYMBOL);
                frame.render_stateful_widget(table, list_area, &mut state);

                let detail = submissions
                    .get(clamp_selection(
                        state.selected().unwrap_or(0),
                        submissions.len(),
                    ))
                    .map(render_submission_detail)
                    .unwrap_or_else(|| vec![Line::from("没有匹配记录")]);
                frame.render_widget(lines_panel("当前摘要", detail), detail_area);
            }

            frame.render_widget(
                footer_panel(
                    submission_footer_text(mode),
                    submission_help_lines(mode),
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
                    return Ok(SyncSelection::Submission(record));
                }
            }
            KeyCode::Char('c') if matches!(mode, SubmissionSelectorMode::Sync) => {
                return Ok(SyncSelection::Chore);
            }
            KeyCode::Char('s') if matches!(mode, SubmissionSelectorMode::Sync) => {
                return Ok(SyncSelection::Skip);
            }
            KeyCode::Esc | KeyCode::Char('q') => return Ok(SyncSelection::Skip),
            _ => {}
        }
    }
}

/// 历史记录选择器，供 `record rebind` 选择要改写的 solve 记录。
pub(crate) fn run_record_app(
    terminal: &mut TerminalHandle,
    problem_id: &str,
    file_name: &str,
    records: &[HistoricalSolveRecord],
) -> Result<Option<HistoricalSolveRecord>> {
    let mut help_visible = false;
    let mut state = TableState::default().with_selected(initial_selection_for_count(records.len()));

    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 4);
            let [list_area, detail_area] = split_main_with_summary(content, 58, 42);
            frame.render_widget(
                lines_panel(
                    "选择要重写的记录",
                    vec![
                        Line::from(format!("题号: {problem_id}")),
                        Line::from(format!("文件: {file_name}")),
                    ],
                ),
                header,
            );

            if records.is_empty() {
                frame.render_widget(
                    text_panel("候选记录", "当前文件没有可重写的 solve 记录"),
                    list_area,
                );
                frame.render_widget(text_panel("当前摘要", "按 Esc 或 q 返回"), detail_area);
            } else {
                let header_row = Row::new([
                    Cell::from("提交时间"),
                    Cell::from("提交 ID"),
                    Cell::from("结果"),
                    Cell::from("Revision"),
                ])
                .style(theme::accent_style().add_modifier(Modifier::BOLD));
                let rows = records.iter().map(build_record_row).collect::<Vec<_>>();
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(16),
                        Constraint::Length(12),
                        Constraint::Length(8),
                        Constraint::Min(12),
                    ],
                )
                .header(header_row)
                .block(crate::ui::terminal::common::panel("候选记录"))
                .row_highlight_style(theme::selected_row_style())
                .highlight_symbol(theme::FOCUS_SYMBOL);
                frame.render_stateful_widget(table, list_area, &mut state);

                let detail = records
                    .get(clamp_selection(
                        state.selected().unwrap_or(0),
                        records.len(),
                    ))
                    .map(render_record_detail)
                    .unwrap_or_else(|| vec![Line::from("没有匹配记录")]);
                frame.render_widget(lines_panel("当前摘要", detail), detail_area);
            }

            frame.render_widget(
                footer_panel(
                    "j/k/↑/↓ 移动  Enter 确认  Esc 返回  q 退出",
                    &[
                        "Enter: 确认当前记录",
                        "Esc: 取消当前选择器并返回",
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
        if let Some(next) = move_selection(key.code, state.selected(), records.len()) {
            state.select(next);
            continue;
        }
        match key.code {
            KeyCode::Enter => {
                return Ok(state
                    .selected()
                    .and_then(|index| records.get(index).cloned()));
            }
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            _ => {}
        }
    }
}

/// 删除确认页。
///
/// 这里只表达“删除还是跳过”，不会直接执行 remove commit。
pub(crate) fn run_delete_app(
    terminal: &mut TerminalHandle,
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
) -> Result<SyncSelection> {
    let mut help_visible = false;
    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 4);
            frame.render_widget(
                lines_panel("确认删除文件", problem_header_lines(problem_id, metadata)),
                header,
            );
            frame.render_widget(
                text_panel(
                    "删除确认",
                    "检测到该题目文件已被删除\nEnter 确认删除\nEsc 或 q 跳过",
                ),
                content,
            );
            frame.render_widget(
                footer_panel(
                    "Enter 确认删除  Esc 跳过  q 退出",
                    &["Enter: 记为 remove", "Esc/q: 跳过当前删除动作"],
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
            KeyCode::Enter => return Ok(SyncSelection::Delete),
            KeyCode::Esc | KeyCode::Char('q') => return Ok(SyncSelection::Skip),
            _ => {}
        }
    }
}

/// 根据调用场景切换页面标题和动作语义。
fn selector_title(mode: SubmissionSelectorMode) -> &'static str {
    match mode {
        SubmissionSelectorMode::Sync => "选择同步结果",
        SubmissionSelectorMode::Record => "选择提交记录",
    }
}

/// 题目头部摘要。
///
/// 能力上只负责把问题上下文整理成两行，不在这里做业务推断。
fn problem_header_lines(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
) -> Vec<Line<'static>> {
    metadata.map_or_else(
        || {
            vec![
                Line::from(format!("题号: {problem_id}")),
                Line::from("难度: -  标签: -"),
            ]
        },
        |item| {
            vec![
                Line::from(format!("题号: {problem_id}  标题: {}", item.title)),
                Line::from(format!(
                    "难度: {}  标签: {}",
                    item.difficulty.as_deref().unwrap_or("-"),
                    if item.tags.is_empty() {
                        "-".to_string()
                    } else {
                        item.tags.join(", ")
                    }
                )),
            ]
        },
    )
}

/// 把 submission 记录映射为表格行。
fn build_submission_row(record: &SubmissionRecord) -> Row<'static> {
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
}

/// 把历史 solve 记录映射为 rebind 选择器的表格行。
fn build_record_row(record: &HistoricalSolveRecord) -> Row<'static> {
    Row::new([
        Cell::from(
            record
                .record
                .submission_time
                .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "---- -- -- --:--".to_string()),
        ),
        Cell::from(
            record
                .record
                .submission_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
        Cell::from(normalize_verdict(&record.record.verdict).into_owned())
            .style(theme::verdict_style(&record.record.verdict)),
        Cell::from(short_revision(&record.revision)),
    ])
}

/// 渲染右侧 submission 摘要区。
fn render_submission_detail(record: &SubmissionRecord) -> Vec<Line<'static>> {
    let verdict = normalize_verdict(&record.verdict).into_owned();
    vec![
        Line::from(vec![
            Span::raw("结果: "),
            Span::styled(verdict.clone(), theme::verdict_style(&verdict)),
        ]),
        Line::from(format!("提交 ID: {}", record.submission_id)),
        Line::from(format!("用户: {}", record.submitter)),
        Line::from(format!(
            "提交时间: {}",
            record
                .submitted_at
                .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!(
            "分数: {}",
            record
                .score
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!(
            "耗时: {}",
            record
                .time_ms
                .map(|value| format!("{value}ms"))
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!(
            "内存: {}",
            record
                .memory_mb
                .map(|value| format!("{value:.1}MB"))
                .unwrap_or_else(|| "-".to_string()),
        )),
    ]
}

/// 渲染右侧历史记录摘要区。
fn render_record_detail(record: &HistoricalSolveRecord) -> Vec<Line<'static>> {
    let verdict = normalize_verdict(&record.record.verdict).into_owned();
    vec![
        Line::from(format!("Revision: {}", short_revision(&record.revision))),
        Line::from(vec![
            Span::raw("结果: "),
            Span::styled(verdict.clone(), theme::verdict_style(&verdict)),
        ]),
        Line::from(format!(
            "提交 ID: {}",
            record
                .record
                .submission_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!(
            "提交时间: {}",
            record
                .record
                .submission_time
                .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!("文件: {}", record.record.file_name)),
    ]
}

/// 不同选择模式下的默认 footer 文案。
fn submission_footer_text(mode: SubmissionSelectorMode) -> &'static str {
    match mode {
        SubmissionSelectorMode::Sync => {
            "j/k/↑/↓ 移动  Enter 确认  c 标记 chore  s 跳过  Esc/q 返回"
        }
        SubmissionSelectorMode::Record => "j/k/↑/↓ 移动  Enter 确认  Esc 返回  q 退出",
    }
}

/// 不同选择模式下的帮助区文案。
fn submission_help_lines(mode: SubmissionSelectorMode) -> &'static [&'static str] {
    match mode {
        SubmissionSelectorMode::Sync => &[
            "Enter: 绑定当前 submission",
            "c: 将当前项标记为 chore",
            "s: 跳过当前项",
            "Esc/q: 返回或退出当前选择器",
        ],
        SubmissionSelectorMode::Record => &["Enter: 绑定当前 submission", "Esc/q: 取消当前选择器"],
    }
}

/// 空状态说明文案。
fn submission_empty_state_text(mode: SubmissionSelectorMode) -> &'static str {
    match mode {
        SubmissionSelectorMode::Sync => "未找到提交记录\n按 c 标记 chore，按 s 跳过",
        SubmissionSelectorMode::Record => "未找到提交记录\n按 Esc 或 q 返回",
    }
}

/// 缩短 revision，避免表格列被长哈希撑爆。
fn short_revision(revision: &str) -> String {
    revision.chars().take(12).collect()
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};
    use crossterm::event::KeyCode;

    use super::{
        SubmissionSelectorMode, build_record_row, build_submission_row, problem_header_lines,
        short_revision, submission_empty_state_text, submission_footer_text,
    };
    use crate::models::{HistoricalSolveRecord, ProblemMetadata, SubmissionRecord, TrainingFields};

    fn sample_metadata() -> ProblemMetadata {
        ProblemMetadata {
            id: "P1001".to_string(),
            title: "A+B Problem".to_string(),
            difficulty: Some("入门".to_string()),
            tags: vec!["模拟".to_string()],
            source: Some("Luogu".to_string()),
            url: "https://www.luogu.com.cn/problem/P1001".to_string(),
            fetched_at: FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                .single()
                .unwrap(),
        }
    }

    #[test]
    fn problem_header_lines_include_problem_title_when_metadata_exists() {
        let header = problem_header_lines("P1001", Some(&sample_metadata()));
        let rendered = format!("{header:?}");

        assert!(rendered.contains("A+B Problem"));
        assert!(rendered.contains("模拟"));
    }

    #[test]
    fn build_submission_row_includes_user_and_submission_id_columns() {
        let row = build_submission_row(&SubmissionRecord {
            submission_id: 1,
            problem_id: Some("P1001".to_string()),
            submitter: "alice".to_string(),
            verdict: "AC".to_string(),
            score: Some(100),
            time_ms: Some(50),
            memory_mb: Some(1.2),
            submitted_at: None,
        });
        let debug_row = format!("{row:?}");

        assert!(debug_row.contains("alice"));
        assert!(debug_row.contains("1"));
        assert!(debug_row.contains("AC"));
    }

    #[test]
    fn build_record_row_includes_revision_and_submission_id() {
        let row = build_record_row(&HistoricalSolveRecord {
            revision: "abcdef1234567890".to_string(),
            record: crate::models::SolveRecord {
                problem_id: "P1001".to_string(),
                title: "A+B Problem".to_string(),
                verdict: "WA".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string()],
                source: "Luogu".to_string(),
                submission_id: Some(1),
                submission_time: None,
                file_name: "P1001.cpp".to_string(),
                training: TrainingFields::default(),
                source_order: 1,
            },
        });
        let debug_row = format!("{row:?}");

        assert!(debug_row.contains("abcdef123456"));
        assert!(debug_row.contains("1"));
        assert!(debug_row.contains("WA"));
    }

    #[test]
    fn selector_texts_reflect_new_workflow_language() {
        assert!(submission_footer_text(SubmissionSelectorMode::Sync).contains("s 跳过"));
        assert!(submission_empty_state_text(SubmissionSelectorMode::Record).contains("Esc 或 q"));
        assert_eq!(short_revision("abcdef1234567890"), "abcdef123456");
        assert!(matches!(KeyCode::Char('q'), KeyCode::Char('q')));
    }
}
