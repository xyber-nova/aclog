use std::{
    io::{self, Stdout},
    path::Path,
};

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::models::{
    BrowserQuery, BrowserRootView, HistoricalSolveRecord, ProblemMetadata, RecordIndex,
    StatsDashboard, StatsSummary, SubmissionRecord, SyncBatchSession, SyncChangeKind,
    SyncSelection, SyncSessionChoice, SyncSessionItem,
};

pub fn select_submission(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncSelection> {
    run_in_terminal(|terminal| {
        run_submission_app(
            terminal,
            problem_id,
            metadata,
            submissions,
            SubmissionSelectorMode::Sync,
        )
    })
}

pub fn select_record_submission(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<Option<SubmissionRecord>> {
    let selection = run_in_terminal(|terminal| {
        run_submission_app(
            terminal,
            problem_id,
            metadata,
            submissions,
            SubmissionSelectorMode::Record,
        )
    })?;
    match selection {
        SyncSelection::Submission(record) => Ok(Some(record)),
        SyncSelection::Skip => Ok(None),
        SyncSelection::Chore | SyncSelection::Delete => Ok(None),
    }
}

pub fn select_record_to_rebind(
    problem_id: &str,
    file_name: &str,
    records: &[HistoricalSolveRecord],
) -> Result<Option<HistoricalSolveRecord>> {
    run_in_terminal(|terminal| run_record_app(terminal, problem_id, file_name, records))
}

pub fn confirm_deleted_file(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
) -> Result<SyncSelection> {
    run_in_terminal(|terminal| run_delete_app(terminal, problem_id, metadata))
}

pub fn show_stats(workspace_root: &Path, summary: &StatsSummary) -> Result<()> {
    run_in_terminal(|terminal| run_stats_app(terminal, workspace_root, summary))
}

pub fn choose_sync_session_action(
    workspace_root: &Path,
    session: &SyncBatchSession,
) -> Result<SyncSessionChoice> {
    run_in_terminal(|terminal| run_sync_session_choice_app(terminal, workspace_root, session))
}

pub fn review_sync_batch(
    workspace_root: &Path,
    session: &SyncBatchSession,
) -> Result<Option<usize>> {
    run_in_terminal(|terminal| run_sync_batch_review_app(terminal, workspace_root, session))
}

pub fn select_sync_batch_action(
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncSelection> {
    run_in_terminal(|terminal| run_sync_item_app(terminal, item, metadata, submissions))
}

pub fn open_record_browser(
    workspace_root: &Path,
    query: &BrowserQuery,
    index: &RecordIndex,
) -> Result<()> {
    run_in_terminal(|terminal| run_browser_app(terminal, workspace_root, query, index))
}

pub fn show_stats_dashboard(
    workspace_root: &Path,
    dashboard: &StatsDashboard,
    index: &RecordIndex,
) -> Result<()> {
    run_in_terminal(|terminal| run_stats_dashboard_app(terminal, workspace_root, dashboard, index))
}

fn run_in_terminal<T>(
    run: impl FnOnce(&mut Terminal<CrosstermBackend<Stdout>>) -> Result<T>,
) -> Result<T> {
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

fn run_submission_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
    mode: SubmissionSelectorMode,
) -> Result<SyncSelection> {
    let mut state = TableState::default().with_selected(initial_selection(submissions));
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area);

            let header = build_problem_header(problem_id, metadata, "选择提交记录");
            let block = Block::default().borders(Borders::ALL);
            let footer = Paragraph::new(submission_footer_text(mode))
                .block(Block::default().borders(Borders::ALL));

            frame.render_widget(header, chunks[0]);
            if submissions.is_empty() {
                let empty_state = Paragraph::new(submission_empty_state_text(mode)).block(block);
                frame.render_widget(empty_state, chunks[1]);
            } else {
                let header_row = Row::new([
                    Cell::from("提交时间"),
                    Cell::from("提交用户"),
                    Cell::from("提交 ID"),
                    Cell::from("结果"),
                    Cell::from("分数"),
                    Cell::from("耗时"),
                    Cell::from("内存"),
                ])
                .style(Style::default().add_modifier(Modifier::BOLD));

                let rows = submissions
                    .iter()
                    .map(build_submission_row)
                    .collect::<Vec<_>>();

                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(16),
                        Constraint::Length(18),
                        Constraint::Length(10),
                        Constraint::Length(10),
                        Constraint::Length(8),
                        Constraint::Length(8),
                        Constraint::Length(8),
                    ],
                )
                .header(header_row)
                .block(block)
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");
                frame.render_stateful_widget(table, chunks[1], &mut state);
            }
            frame.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match apply_submission_key_code(key.code, state.selected(), submissions, mode) {
                SelectionOutcome::Continue(next_selection) => state.select(next_selection),
                SelectionOutcome::Select(selection) => return Ok(selection),
                SelectionOutcome::Ignore => {}
            }
        }
    }
}

fn run_record_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    problem_id: &str,
    file_name: &str,
    records: &[HistoricalSolveRecord],
) -> Result<Option<HistoricalSolveRecord>> {
    let mut state = TableState::default().with_selected(initial_selection_for_records(records));
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(6),
                Constraint::Length(3),
            ])
            .split(area);

            let header = Paragraph::new(vec![
                Line::from(format!("题号: {problem_id}")),
                Line::from(format!("文件: {file_name}")),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("选择要重写的记录"),
            );
            let footer = Paragraph::new("↑/↓ 移动  Enter 确认  Esc 取消")
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            if records.is_empty() {
                frame.render_widget(
                    Paragraph::new("当前文件没有可重写的 solve 记录\n按 Esc 返回")
                        .block(Block::default().borders(Borders::ALL)),
                    chunks[1],
                );
            } else {
                let header_row = Row::new([
                    Cell::from("提交时间"),
                    Cell::from("提交 ID"),
                    Cell::from("结果"),
                    Cell::from("Revision"),
                ])
                .style(Style::default().add_modifier(Modifier::BOLD));
                let rows = records.iter().map(build_record_row).collect::<Vec<_>>();
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(16),
                        Constraint::Length(12),
                        Constraint::Length(10),
                        Constraint::Min(12),
                    ],
                )
                .header(header_row)
                .block(Block::default().borders(Borders::ALL))
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");
                frame.render_stateful_widget(table, chunks[1], &mut state);
            }

            frame.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match apply_record_key_code(key.code, state.selected(), records) {
                RecordSelectionOutcome::Continue(next_selection) => state.select(next_selection),
                RecordSelectionOutcome::Select(index) => {
                    return Ok(index.and_then(|item| records.get(item).cloned()));
                }
                RecordSelectionOutcome::Ignore => {}
            }
        }
    }
}

fn run_delete_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
) -> Result<SyncSelection> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area);

            let header = build_problem_header(problem_id, metadata, "确认删除文件");
            let body = Paragraph::new("检测到该题目文件已被删除\n按 Enter 确认删除，按 Esc 跳过")
                .block(Block::default().borders(Borders::ALL));
            let footer = Paragraph::new("Enter 确认删除  Esc 跳过")
                .block(Block::default().borders(Borders::ALL));

            frame.render_widget(header, chunks[0]);
            frame.render_widget(body, chunks[1]);
            frame.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match apply_delete_key_code(key.code) {
                SelectionOutcome::Select(selection) => return Ok(selection),
                SelectionOutcome::Continue(_) | SelectionOutcome::Ignore => {}
            }
        }
    }
}

fn run_stats_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    workspace_root: &Path,
    summary: &StatsSummary,
) -> Result<()> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);

            let detail_chunks = Layout::horizontal([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(chunks[2]);

            let header = Paragraph::new(stats_header_lines(workspace_root, summary))
                .block(Block::default().borders(Borders::ALL).title("做题情况统计"));
            let overview = Paragraph::new(stats_overview_lines(summary))
                .block(Block::default().borders(Borders::ALL).title("总体概览"));
            let footer =
                Paragraph::new("q / Esc 退出").block(Block::default().borders(Borders::ALL));

            frame.render_widget(header, chunks[0]);
            frame.render_widget(overview, chunks[1]);
            match stats_content_mode(summary) {
                StatsContentMode::Empty => {
                    let empty_state = Paragraph::new(stats_empty_state_lines())
                        .block(Block::default().borders(Borders::ALL).title("空状态"));
                    frame.render_widget(empty_state, chunks[2]);
                }
                StatsContentMode::Distributions => {
                    let verdicts = Paragraph::new(distribution_lines(
                        &summary.verdict_counts,
                        "当前工作区本地 solve 记录的结果分布",
                    ))
                    .block(Block::default().borders(Borders::ALL).title("结果分布"));
                    let difficulties = Paragraph::new(distribution_lines(
                        &summary.difficulty_counts,
                        "按题号去重后的最新记录难度分布",
                    ))
                    .block(Block::default().borders(Borders::ALL).title("难度分布"));
                    let tags = Paragraph::new(distribution_lines(
                        &summary.tag_counts,
                        "按题号去重后的最新记录算法标签分布",
                    ))
                    .block(Block::default().borders(Borders::ALL).title("标签分布"));
                    frame.render_widget(verdicts, detail_chunks[0]);
                    frame.render_widget(difficulties, detail_chunks[1]);
                    frame.render_widget(tags, detail_chunks[2]);
                }
            }
            frame.render_widget(footer, chunks[3]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if should_exit_stats(key.code) {
                return Ok(());
            }
        }
    }
}

fn run_sync_session_choice_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    workspace_root: &Path,
    session: &SyncBatchSession,
) -> Result<SyncSessionChoice> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(6),
                Constraint::Length(3),
            ])
            .split(area);

            let header = Paragraph::new(vec![
                Line::from(format!("工作区: {}", workspace_root.display())),
                Line::from(format!(
                    "检测到未完成 sync 批次，待处理项 {} 个",
                    session
                        .items
                        .iter()
                        .filter(|item| matches!(
                            item.status,
                            crate::models::SyncItemStatus::Pending
                        ))
                        .count()
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title("恢复批次"));
            let body = Paragraph::new("按 r 继续恢复，按 n 重建当前批次")
                .block(Block::default().borders(Borders::ALL));
            let footer =
                Paragraph::new("r 恢复  n 重建").block(Block::default().borders(Borders::ALL));

            frame.render_widget(header, chunks[0]);
            frame.render_widget(body, chunks[1]);
            frame.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('r') => return Ok(SyncSessionChoice::Resume),
                KeyCode::Char('n') => return Ok(SyncSessionChoice::Rebuild),
                _ => {}
            }
        }
    }
}

fn run_sync_batch_review_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    workspace_root: &Path,
    session: &SyncBatchSession,
) -> Result<Option<usize>> {
    let mut state =
        TableState::default().with_selected(initial_selection_for_count(session.items.len()));
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);
            let header = Paragraph::new(vec![
                Line::from(format!("工作区: {}", workspace_root.display())),
                Line::from("从批次预览中选择要处理的文件"),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Sync 批次预览"),
            );
            let footer = Paragraph::new("↑/↓ 移动  Enter 进入详情  Esc 暂停并保留批次")
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            let header_row = Row::new([
                Cell::from("文件"),
                Cell::from("题号"),
                Cell::from("类型"),
                Cell::from("状态"),
                Cell::from("提交数"),
                Cell::from("默认候选"),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD));
            let rows = session
                .items
                .iter()
                .map(|item| {
                    Row::new([
                        Cell::from(item.file.clone()),
                        Cell::from(item.problem_id.clone().unwrap_or_else(|| "-".to_string())),
                        Cell::from(match item.kind {
                            SyncChangeKind::Active => "已修改",
                            SyncChangeKind::Deleted => "已删除",
                        }),
                        Cell::from(match item.status {
                            crate::models::SyncItemStatus::Pending => "待处理",
                            crate::models::SyncItemStatus::Planned => "已决待提交",
                            crate::models::SyncItemStatus::Skipped => "已跳过",
                            crate::models::SyncItemStatus::Committed => "已提交",
                            crate::models::SyncItemStatus::Invalid => "已失效",
                        }),
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
                    Constraint::Percentage(32),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(12),
                    Constraint::Length(10),
                    Constraint::Length(12),
                ],
            )
            .header(header_row)
            .block(Block::default().borders(Borders::ALL))
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
            frame.render_stateful_widget(table, chunks[1], &mut state);
            frame.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match apply_index_key_code(key.code, state.selected(), session.items.len()) {
                IndexSelectionOutcome::Continue(next) => state.select(next),
                IndexSelectionOutcome::Select(index) => return Ok(index),
                IndexSelectionOutcome::Ignore => {}
            }
        }
    }
}

fn run_sync_item_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncSelection> {
    match item.kind {
        SyncChangeKind::Deleted => run_sync_delete_app(terminal, item, metadata),
        SyncChangeKind::Active => run_sync_submission_app(terminal, item, metadata, submissions),
    }
}

fn run_sync_delete_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
) -> Result<SyncSelection> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(5),
                Constraint::Min(6),
                Constraint::Length(3),
            ])
            .split(area);
            let header = Paragraph::new(sync_item_header_lines(item, metadata))
                .block(Block::default().borders(Borders::ALL).title("确认删除"));
            let body = Paragraph::new("检测到题解文件已删除\n按 Enter 记为 remove，按 Esc 跳过")
                .block(Block::default().borders(Borders::ALL));
            let footer = Paragraph::new("Enter 确认删除  Esc 跳过")
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);
            frame.render_widget(body, chunks[1]);
            frame.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match apply_delete_key_code(key.code) {
                SelectionOutcome::Select(selection) => return Ok(selection),
                SelectionOutcome::Continue(_) | SelectionOutcome::Ignore => {}
            }
        }
    }
}

fn run_sync_submission_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncSelection> {
    let mut state = TableState::default().with_selected(initial_selection(submissions));
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(6),
                Constraint::Min(6),
                Constraint::Length(3),
            ])
            .split(area);
            let header = Paragraph::new(sync_item_header_lines(item, metadata))
                .block(Block::default().borders(Borders::ALL).title("选择同步结果"));
            let footer = Paragraph::new("↑/↓ 移动  Enter 选择 submission  c 标记 chore  Esc 跳过")
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            if submissions.is_empty() {
                let body = Paragraph::new("未找到提交记录\n按 c 标记 chore，按 Esc 跳过")
                    .block(Block::default().borders(Borders::ALL).title("空状态"));
                frame.render_widget(body, chunks[1]);
            } else {
                let header_row = Row::new([
                    Cell::from("提交时间"),
                    Cell::from("提交用户"),
                    Cell::from("提交 ID"),
                    Cell::from("结果"),
                    Cell::from("分数"),
                    Cell::from("耗时"),
                    Cell::from("内存"),
                ])
                .style(Style::default().add_modifier(Modifier::BOLD));
                let rows = submissions
                    .iter()
                    .map(build_submission_row)
                    .collect::<Vec<_>>();
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(16),
                        Constraint::Length(18),
                        Constraint::Length(10),
                        Constraint::Length(10),
                        Constraint::Length(8),
                        Constraint::Length(8),
                        Constraint::Length(8),
                    ],
                )
                .header(header_row)
                .block(Block::default().borders(Borders::ALL))
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");
                frame.render_stateful_widget(table, chunks[1], &mut state);
            }

            frame.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match apply_submission_key_code(
                key.code,
                state.selected(),
                submissions,
                SubmissionSelectorMode::Sync,
            ) {
                SelectionOutcome::Continue(next_selection) => state.select(next_selection),
                SelectionOutcome::Select(selection) => return Ok(selection),
                SelectionOutcome::Ignore => {}
            }
        }
    }
}

fn run_browser_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    workspace_root: &Path,
    query: &BrowserQuery,
    index: &RecordIndex,
) -> Result<()> {
    use crate::domain::browser::{
        build_browser_state, filter_browser_files, filter_browser_problems, filter_timeline_rows,
        timeline_rows_for_file, timeline_rows_for_problem,
    };

    #[derive(Clone)]
    enum BrowserScreen {
        Files,
        Problems,
        FileTimeline(String),
        ProblemTimeline(String),
    }

    let state = build_browser_state(index);
    let mut screen = if let Some(file_name) = query.timeline_file.as_ref() {
        BrowserScreen::FileTimeline(file_name.clone())
    } else if let Some(problem_id) = query.timeline_problem.as_ref() {
        BrowserScreen::ProblemTimeline(problem_id.clone())
    } else {
        match query.root_view {
            BrowserRootView::Files => BrowserScreen::Files,
            BrowserRootView::Problems => BrowserScreen::Problems,
        }
    };
    let mut selected = 0usize;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);
            let content = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(chunks[1]);

            let header = Paragraph::new(vec![
                Line::from(format!("工作区: {}", workspace_root.display())),
                Line::from(browser_query_summary(query)),
            ])
            .block(Block::default().borders(Borders::ALL).title("记录浏览工作台"));
            frame.render_widget(header, chunks[0]);

            match &screen {
                BrowserScreen::Files => {
                    let rows = filter_browser_files(&state.files, query);
                    let capped = clamp_selection(selected, rows.len());
                    let table = Table::new(
                        rows.iter().map(|row| {
                            Row::new([
                                Cell::from(row.file_name.clone()),
                                Cell::from(row.problem_id.clone()),
                                Cell::from(row.verdict.clone()).style(verdict_style(&row.verdict)),
                                Cell::from(
                                    row.submission_time
                                        .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| "-".to_string()),
                                ),
                            ])
                        }).collect::<Vec<_>>(),
                        [
                            Constraint::Percentage(44),
                            Constraint::Length(10),
                            Constraint::Length(8),
                            Constraint::Length(16),
                        ],
                    )
                    .header(
                        Row::new(["文件", "题号", "结果", "记录时间"])
                            .style(Style::default().add_modifier(Modifier::BOLD)),
                    )
                    .block(Block::default().borders(Borders::ALL).title("文件视角"))
                    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("> ");
                    let mut table_state = TableState::default().with_selected(initial_selection_for_count(rows.len()).map(|_| capped));
                    frame.render_stateful_widget(table, content[0], &mut table_state);

                    let detail = rows
                        .get(capped)
                        .and_then(|row| index.timeline_for_file(&row.file_name).first())
                        .map(crate::app::support::render_record_detail)
                        .unwrap_or_else(|| "没有匹配记录".to_string());
                    frame.render_widget(
                        Paragraph::new(detail).block(Block::default().borders(Borders::ALL).title("详情")),
                        content[1],
                    );
                }
                BrowserScreen::Problems => {
                    let rows = filter_browser_problems(&state.problems, query);
                    let capped = clamp_selection(selected, rows.len());
                    let table = Table::new(
                        rows.iter().map(|row| {
                            Row::new([
                                Cell::from(row.problem_id.clone()),
                                Cell::from(row.verdict.clone()).style(verdict_style(&row.verdict)),
                                Cell::from(row.files.len().to_string()),
                                Cell::from(
                                    row.submission_time
                                        .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| "-".to_string()),
                                ),
                            ])
                        }).collect::<Vec<_>>(),
                        [
                            Constraint::Length(10),
                            Constraint::Length(8),
                            Constraint::Length(8),
                            Constraint::Length(16),
                        ],
                    )
                    .header(
                        Row::new(["题号", "结果", "文件数", "记录时间"])
                            .style(Style::default().add_modifier(Modifier::BOLD)),
                    )
                    .block(Block::default().borders(Borders::ALL).title("题目视角"))
                    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("> ");
                    let mut table_state = TableState::default().with_selected(initial_selection_for_count(rows.len()).map(|_| capped));
                    frame.render_stateful_widget(table, content[0], &mut table_state);

                    let detail = rows
                        .get(capped)
                        .map(|row| {
                            format!(
                                "题号: {}\n标题: {}\n结果: {}\n难度: {}\n文件: {}\n标签: {}\n训练摘要: {}",
                                row.problem_id,
                                row.title,
                                row.verdict,
                                row.difficulty,
                                if row.files.is_empty() { "-".to_string() } else { row.files.join(", ") },
                                if row.tags.is_empty() { "-".to_string() } else { row.tags.join(", ") },
                                row.training_summary,
                            )
                        })
                        .unwrap_or_else(|| "没有匹配记录".to_string());
                    frame.render_widget(
                        Paragraph::new(detail).block(Block::default().borders(Borders::ALL).title("详情")),
                        content[1],
                    );
                }
                BrowserScreen::FileTimeline(file_name) => {
                    let rows = filter_timeline_rows(&timeline_rows_for_file(index, file_name), query);
                    let capped = clamp_selection(selected, rows.len());
                    let table = Table::new(
                        rows.iter().map(|row| {
                            Row::new([
                                Cell::from(
                                    row.submission_time
                                        .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| "-".to_string()),
                                ),
                                Cell::from(row.verdict.clone()).style(verdict_style(&row.verdict)),
                                Cell::from(short_revision(&row.revision)),
                            ])
                        }).collect::<Vec<_>>(),
                        [Constraint::Length(16), Constraint::Length(8), Constraint::Length(14)],
                    )
                    .header(
                        Row::new(["提交时间", "结果", "Revision"])
                            .style(Style::default().add_modifier(Modifier::BOLD)),
                    )
                    .block(Block::default().borders(Borders::ALL).title(format!("文件时间线: {file_name}")))
                    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("> ");
                    let mut table_state = TableState::default().with_selected(initial_selection_for_count(rows.len()).map(|_| capped));
                    frame.render_stateful_widget(table, content[0], &mut table_state);
                    let detail = rows
                        .get(capped)
                        .and_then(|row| index.timeline_for_file(file_name).iter().find(|record| record.revision == row.revision))
                        .map(crate::app::support::render_record_detail)
                        .unwrap_or_else(|| "没有匹配记录".to_string());
                    frame.render_widget(
                        Paragraph::new(detail).block(Block::default().borders(Borders::ALL).title("记录详情")),
                        content[1],
                    );
                }
                BrowserScreen::ProblemTimeline(problem_id) => {
                    let rows = filter_timeline_rows(&timeline_rows_for_problem(index, problem_id), query);
                    let capped = clamp_selection(selected, rows.len());
                    let table = Table::new(
                        rows.iter().map(|row| {
                            Row::new([
                                Cell::from(
                                    row.submission_time
                                        .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| "-".to_string()),
                                ),
                                Cell::from(row.file_name.clone()),
                                Cell::from(row.verdict.clone()).style(verdict_style(&row.verdict)),
                            ])
                        }).collect::<Vec<_>>(),
                        [Constraint::Length(16), Constraint::Percentage(55), Constraint::Length(8)],
                    )
                    .header(
                        Row::new(["提交时间", "文件", "结果"])
                            .style(Style::default().add_modifier(Modifier::BOLD)),
                    )
                    .block(Block::default().borders(Borders::ALL).title(format!("题目时间线: {problem_id}")))
                    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("> ");
                    let mut table_state = TableState::default().with_selected(initial_selection_for_count(rows.len()).map(|_| capped));
                    frame.render_stateful_widget(table, content[0], &mut table_state);
                    let detail = rows
                        .get(capped)
                        .and_then(|row| index.timeline_for_problem(problem_id).iter().find(|record| record.revision == row.revision))
                        .map(crate::app::support::render_record_detail)
                        .unwrap_or_else(|| "没有匹配记录".to_string());
                    frame.render_widget(
                        Paragraph::new(detail).block(Block::default().borders(Borders::ALL).title("记录详情")),
                        content[1],
                    );
                }
            }

            let footer = Paragraph::new(match screen {
                BrowserScreen::Files | BrowserScreen::Problems => {
                    "↑/↓ 移动  Tab 切换视角  Enter 打开时间线  q / Esc 退出"
                }
                BrowserScreen::FileTimeline(_) | BrowserScreen::ProblemTimeline(_) => {
                    "↑/↓ 移动  b 返回  q / Esc 退出"
                }
            })
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match &mut screen {
                BrowserScreen::Files => match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => {
                        let rows = filter_browser_files(&state.files, query);
                        selected = (selected + 1).min(rows.len().saturating_sub(1));
                    }
                    KeyCode::Tab => {
                        screen = BrowserScreen::Problems;
                        selected = 0;
                    }
                    KeyCode::Enter => {
                        let rows = filter_browser_files(&state.files, query);
                        if let Some(row) = rows.get(clamp_selection(selected, rows.len())) {
                            screen = BrowserScreen::FileTimeline(row.file_name.clone());
                            selected = 0;
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ => {}
                },
                BrowserScreen::Problems => match key.code {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => {
                        let rows = filter_browser_problems(&state.problems, query);
                        selected = (selected + 1).min(rows.len().saturating_sub(1));
                    }
                    KeyCode::Tab => {
                        screen = BrowserScreen::Files;
                        selected = 0;
                    }
                    KeyCode::Enter => {
                        let rows = filter_browser_problems(&state.problems, query);
                        if let Some(row) = rows.get(clamp_selection(selected, rows.len())) {
                            screen = BrowserScreen::ProblemTimeline(row.problem_id.clone());
                            selected = 0;
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ => {}
                },
                BrowserScreen::FileTimeline(_) | BrowserScreen::ProblemTimeline(_) => {
                    match key.code {
                        KeyCode::Up => selected = selected.saturating_sub(1),
                        KeyCode::Down => selected = selected.saturating_add(1),
                        KeyCode::Char('b') => {
                            screen = match query.root_view {
                                BrowserRootView::Files => BrowserScreen::Files,
                                BrowserRootView::Problems => BrowserScreen::Problems,
                            };
                            selected = 0;
                        }
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn run_stats_dashboard_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    workspace_root: &Path,
    dashboard: &StatsDashboard,
    index: &RecordIndex,
) -> Result<()> {
    let mut review_selected = 0usize;
    let mut review_mode = dashboard.start_in_review;
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);

            let header = Paragraph::new(stats_header_lines(workspace_root, &dashboard.summary))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("训练统计与建议"),
                );
            let overview = Paragraph::new(stats_overview_lines(&dashboard.summary))
                .block(Block::default().borders(Borders::ALL).title("总体概览"));
            frame.render_widget(header, chunks[0]);
            frame.render_widget(overview, chunks[1]);

            if review_mode {
                let content =
                    Layout::horizontal([Constraint::Percentage(44), Constraint::Percentage(56)])
                        .split(chunks[2]);
                let rows = dashboard
                    .review_candidates
                    .iter()
                    .map(|item| {
                        Row::new([
                            Cell::from(item.kind.clone()),
                            Cell::from(item.label.clone()),
                            Cell::from(item.verdict.clone().unwrap_or_else(|| "-".to_string())),
                        ])
                    })
                    .collect::<Vec<_>>();
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(12),
                        Constraint::Percentage(56),
                        Constraint::Length(10),
                    ],
                )
                .header(
                    Row::new(["类型", "标签", "结果"])
                        .style(Style::default().add_modifier(Modifier::BOLD)),
                )
                .block(Block::default().borders(Borders::ALL).title("复习建议"))
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");
                let mut table_state = TableState::default().with_selected(
                    initial_selection_for_count(dashboard.review_candidates.len()).map(|_| {
                        clamp_selection(review_selected, dashboard.review_candidates.len())
                    }),
                );
                frame.render_stateful_widget(table, content[0], &mut table_state);

                let detail = dashboard
                    .review_candidates
                    .get(clamp_selection(
                        review_selected,
                        dashboard.review_candidates.len(),
                    ))
                    .map(|item| {
                        format!(
                            "标签: {}\n结果: {}\n原因: {}\n上次时间: {}",
                            item.label,
                            item.verdict.clone().unwrap_or_else(|| "-".to_string()),
                            item.reason,
                            item.last_submission_time
                                .map(|value| value.to_rfc3339())
                                .unwrap_or_else(|| "-".to_string())
                        )
                    })
                    .unwrap_or_else(|| "当前没有可用的复习建议".to_string());
                frame.render_widget(
                    Paragraph::new(detail)
                        .block(Block::default().borders(Borders::ALL).title("建议详情")),
                    content[1],
                );
            } else {
                let detail_chunks = Layout::horizontal([
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ])
                .split(chunks[2]);
                let verdicts = Paragraph::new(distribution_lines(
                    &dashboard.summary.verdict_counts,
                    "当前工作区本地 solve 记录的结果分布",
                ))
                .block(Block::default().borders(Borders::ALL).title("结果分布"));
                let difficulties = Paragraph::new(distribution_lines(
                    &dashboard.summary.difficulty_counts,
                    "按题号去重后的最新记录难度分布",
                ))
                .block(Block::default().borders(Borders::ALL).title("难度分布"));
                let tags = Paragraph::new(distribution_lines(
                    &dashboard.summary.tag_counts,
                    "按题号去重后的最新记录算法标签分布",
                ))
                .block(Block::default().borders(Borders::ALL).title("标签分布"));
                frame.render_widget(verdicts, detail_chunks[0]);
                frame.render_widget(difficulties, detail_chunks[1]);
                frame.render_widget(tags, detail_chunks[2]);
            }

            let footer = Paragraph::new(if review_mode {
                "↑/↓ 移动  Enter 打开对应历史  b 返回统计页  f 文件浏览  p 题目浏览  q / Esc 退出"
            } else {
                "r 查看复习建议  f 文件浏览  p 题目浏览  q / Esc 退出"
            })
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(footer, chunks[3]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc if !review_mode => return Ok(()),
                KeyCode::Char('f') => {
                    run_browser_app(
                        terminal,
                        workspace_root,
                        &BrowserQuery {
                            root_view: BrowserRootView::Files,
                            ..BrowserQuery::default()
                        },
                        index,
                    )?;
                }
                KeyCode::Char('p') => {
                    run_browser_app(
                        terminal,
                        workspace_root,
                        &BrowserQuery {
                            root_view: BrowserRootView::Problems,
                            ..BrowserQuery::default()
                        },
                        index,
                    )?;
                }
                KeyCode::Char('r') if !review_mode => {
                    review_mode = true;
                    review_selected = 0;
                }
                KeyCode::Char('b') if review_mode => review_mode = false,
                KeyCode::Up if review_mode => review_selected = review_selected.saturating_sub(1),
                KeyCode::Down if review_mode => {
                    review_selected = (review_selected + 1)
                        .min(dashboard.review_candidates.len().saturating_sub(1));
                }
                KeyCode::Enter if review_mode => {
                    if let Some(candidate) = dashboard.review_candidates.get(clamp_selection(
                        review_selected,
                        dashboard.review_candidates.len(),
                    )) {
                        let query = if let Some(problem_id) = candidate.problem_id.as_ref() {
                            BrowserQuery {
                                root_view: BrowserRootView::Problems,
                                problem_id: Some(problem_id.clone()),
                                timeline_problem: Some(problem_id.clone()),
                                ..BrowserQuery::default()
                            }
                        } else {
                            BrowserQuery {
                                root_view: BrowserRootView::Problems,
                                tag: Some(candidate.label.clone()),
                                ..BrowserQuery::default()
                            }
                        };
                        run_browser_app(terminal, workspace_root, &query, index)?;
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                _ => {}
            }
        }
    }
}

fn sync_item_header_lines(
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("文件: {}", item.file)),
        Line::from(format!(
            "题号: {}  类型: {}",
            item.problem_id.as_deref().unwrap_or("-"),
            match item.kind {
                SyncChangeKind::Active => "已修改",
                SyncChangeKind::Deleted => "已删除",
            }
        )),
    ];
    if let Some(metadata) = metadata {
        lines.push(Line::from(format!(
            "标题: {}  难度: {}",
            metadata.title,
            metadata.difficulty.as_deref().unwrap_or("-")
        )));
    }
    if !item.warnings.is_empty() {
        for warning in &item.warnings {
            lines.push(Line::from(format!("告警: {}", warning.message)));
        }
    } else if let Some(reason) = item.invalid_reason.as_deref() {
        lines.push(Line::from(format!("状态: {reason}")));
    }
    lines
}

fn browser_query_summary(query: &BrowserQuery) -> String {
    let mut parts = vec![match query.root_view {
        BrowserRootView::Files => "视角: files".to_string(),
        BrowserRootView::Problems => "视角: problems".to_string(),
    }];
    if let Some(problem_id) = query.problem_id.as_deref() {
        parts.push(format!("题号={problem_id}"));
    }
    if let Some(file_name) = query.file_name.as_deref() {
        parts.push(format!("文件={file_name}"));
    }
    if let Some(verdict) = query.verdict.as_deref() {
        parts.push(format!("结果={verdict}"));
    }
    if let Some(difficulty) = query.difficulty.as_deref() {
        parts.push(format!("难度={difficulty}"));
    }
    if let Some(tag) = query.tag.as_deref() {
        parts.push(format!("标签={tag}"));
    }
    if let Some(days) = query.days {
        parts.push(format!("最近 {days} 天"));
    }
    parts.join("  ")
}

fn initial_selection_for_count(count: usize) -> Option<usize> {
    (count > 0).then_some(0)
}

fn clamp_selection(selected: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        selected.min(count - 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndexSelectionOutcome {
    Continue(Option<usize>),
    Select(Option<usize>),
    Ignore,
}

fn apply_index_key_code(
    key: KeyCode,
    selected: Option<usize>,
    count: usize,
) -> IndexSelectionOutcome {
    match key {
        KeyCode::Up => {
            let next = selected.unwrap_or(0).saturating_sub(1);
            IndexSelectionOutcome::Continue(initial_selection_for_count(count).map(|_| next))
        }
        KeyCode::Down => {
            let Some(current) = selected else {
                return IndexSelectionOutcome::Ignore;
            };
            IndexSelectionOutcome::Continue(Some((current + 1).min(count.saturating_sub(1))))
        }
        KeyCode::Enter => IndexSelectionOutcome::Select(selected),
        KeyCode::Esc => IndexSelectionOutcome::Select(None),
        _ => IndexSelectionOutcome::Ignore,
    }
}

fn initial_selection(submissions: &[SubmissionRecord]) -> Option<usize> {
    (!submissions.is_empty()).then_some(0)
}

fn initial_selection_for_records(records: &[HistoricalSolveRecord]) -> Option<usize> {
    (!records.is_empty()).then_some(0)
}

fn build_problem_header(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    title: &'static str,
) -> Paragraph<'static> {
    Paragraph::new(problem_header_lines(problem_id, metadata))
        .block(Block::default().borders(Borders::ALL).title(title))
}

fn problem_header_lines(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
) -> Vec<Line<'static>> {
    metadata.map_or_else(
        || {
            vec![
                Line::from(format!("{problem_id}: {problem_id}")),
                Line::from("难度: -  标签: -"),
            ]
        },
        |item| {
            vec![
                Line::from(format!("{problem_id}: {}", item.title)),
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
        Cell::from(record.verdict.clone()).style(verdict_style(&record.verdict)),
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
        Cell::from(record.record.verdict.clone()).style(verdict_style(&record.record.verdict)),
        Cell::from(short_revision(&record.revision)),
    ])
}

fn verdict_style(verdict: &str) -> Style {
    match verdict.trim().to_ascii_uppercase().as_str() {
        "AC" => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        "WA" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        _ => Style::default(),
    }
}

fn stats_header_lines(workspace_root: &Path, summary: &StatsSummary) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("工作区: {}", workspace_root.display())),
        Line::from(format!(
            "统计范围: 本地 jj 历史中的 solve(...) commit  |  记录数: {}",
            summary.total_solve_records
        )),
    ];
    if let Some(days) = summary.time_window_days {
        lines.push(Line::from(format!("时间窗口: 最近 {days} 天")));
    }
    lines
}

fn stats_overview_lines(summary: &StatsSummary) -> Vec<Line<'static>> {
    vec![
        Line::from(format!("唯一题目数: {}", summary.unique_problem_count)),
        Line::from(format!("solve 记录数: {}", summary.total_solve_records)),
        Line::from(format!("唯一题目 AC: {}", summary.unique_ac_count)),
        Line::from(format!("唯一题目非 AC: {}", summary.unique_non_ac_count)),
        Line::from(format!("首次 AC: {}", summary.first_ac_count)),
        Line::from(format!("重复练习题数: {}", summary.repeated_practice_count)),
    ]
}

fn stats_empty_state_lines() -> Vec<Line<'static>> {
    vec![
        Line::from("当前工作区还没有已记录的做题提交"),
        Line::from("请先通过 sync 生成 solve(...) commit，再回来查看统计"),
    ]
}

fn distribution_lines(items: &[(String, usize)], description: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(description.to_string()), Line::from("")];
    if items.is_empty() {
        lines.push(Line::from("无数据"));
        return lines;
    }
    lines.extend(
        items
            .iter()
            .map(|(label, count)| Line::from(format!("{label}: {count}"))),
    );
    lines
}

fn should_exit_stats(key: KeyCode) -> bool {
    matches!(key, KeyCode::Char('q') | KeyCode::Esc)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsContentMode {
    Empty,
    Distributions,
}

fn stats_content_mode(summary: &StatsSummary) -> StatsContentMode {
    if summary.total_solve_records == 0 {
        StatsContentMode::Empty
    } else {
        StatsContentMode::Distributions
    }
}

#[derive(Debug, Clone)]
enum SelectionOutcome {
    Continue(Option<usize>),
    Select(SyncSelection),
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubmissionSelectorMode {
    Sync,
    Record,
}

#[derive(Debug, Clone)]
enum RecordSelectionOutcome {
    Continue(Option<usize>),
    Select(Option<usize>),
    Ignore,
}

fn apply_submission_key_code(
    key: KeyCode,
    selected: Option<usize>,
    submissions: &[SubmissionRecord],
    mode: SubmissionSelectorMode,
) -> SelectionOutcome {
    match key {
        KeyCode::Up => {
            let next = selected.unwrap_or(0).saturating_sub(1);
            SelectionOutcome::Continue(initial_selection(submissions).map(|_| next))
        }
        KeyCode::Down => {
            let Some(current) = selected else {
                return SelectionOutcome::Ignore;
            };
            let max_index = submissions.len().saturating_sub(1);
            SelectionOutcome::Continue(Some((current + 1).min(max_index)))
        }
        KeyCode::Enter => selected
            .and_then(|index| submissions.get(index).cloned())
            .map(|record| SelectionOutcome::Select(SyncSelection::Submission(record)))
            .unwrap_or(SelectionOutcome::Ignore),
        KeyCode::Char('c') if matches!(mode, SubmissionSelectorMode::Sync) => {
            SelectionOutcome::Select(SyncSelection::Chore)
        }
        KeyCode::Esc => SelectionOutcome::Select(match mode {
            SubmissionSelectorMode::Sync => SyncSelection::Skip,
            SubmissionSelectorMode::Record => SyncSelection::Skip,
        }),
        _ => SelectionOutcome::Ignore,
    }
}

fn apply_record_key_code(
    key: KeyCode,
    selected: Option<usize>,
    records: &[HistoricalSolveRecord],
) -> RecordSelectionOutcome {
    match key {
        KeyCode::Up => {
            let next = selected.unwrap_or(0).saturating_sub(1);
            RecordSelectionOutcome::Continue(initial_selection_for_records(records).map(|_| next))
        }
        KeyCode::Down => {
            let Some(current) = selected else {
                return RecordSelectionOutcome::Ignore;
            };
            let max_index = records.len().saturating_sub(1);
            RecordSelectionOutcome::Continue(Some((current + 1).min(max_index)))
        }
        KeyCode::Enter => RecordSelectionOutcome::Select(selected),
        KeyCode::Esc => RecordSelectionOutcome::Select(None),
        _ => RecordSelectionOutcome::Ignore,
    }
}

fn submission_footer_text(mode: SubmissionSelectorMode) -> &'static str {
    match mode {
        SubmissionSelectorMode::Sync => "↑/↓ 移动  Enter 确认  c 标记chore  Esc 跳过",
        SubmissionSelectorMode::Record => "↑/↓ 移动  Enter 确认  Esc 取消",
    }
}

fn submission_empty_state_text(mode: SubmissionSelectorMode) -> &'static str {
    match mode {
        SubmissionSelectorMode::Sync => "未找到提交记录\n按 c 标记 chore，按 Esc 跳过",
        SubmissionSelectorMode::Record => "未找到提交记录\n按 Esc 返回",
    }
}

fn short_revision(revision: &str) -> String {
    revision.chars().take(12).collect()
}

fn apply_delete_key_code(key: KeyCode) -> SelectionOutcome {
    match key {
        KeyCode::Enter => SelectionOutcome::Select(SyncSelection::Delete),
        KeyCode::Esc => SelectionOutcome::Select(SyncSelection::Skip),
        _ => SelectionOutcome::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};
    use crossterm::event::KeyCode;
    use ratatui::style::{Color, Modifier, Style};

    use super::{
        RecordSelectionOutcome, SelectionOutcome, StatsContentMode, SubmissionSelectorMode,
        apply_delete_key_code, apply_record_key_code, apply_submission_key_code, build_record_row,
        build_submission_row, distribution_lines, initial_selection, initial_selection_for_records,
        problem_header_lines, short_revision, should_exit_stats, stats_content_mode,
        stats_empty_state_lines, stats_header_lines, stats_overview_lines,
        submission_empty_state_text, submission_footer_text, verdict_style,
    };
    use crate::models::{
        HistoricalSolveRecord, ProblemMetadata, StatsSummary, SubmissionRecord, TrainingFields,
    };

    fn sample_submissions() -> Vec<SubmissionRecord> {
        vec![
            SubmissionRecord {
                submission_id: 1,
                problem_id: Some("P1001".to_string()),
                submitter: "alice".to_string(),
                verdict: "AC".to_string(),
                score: Some(100),
                time_ms: Some(50),
                memory_mb: Some(1.2),
                submitted_at: None,
            },
            SubmissionRecord {
                submission_id: 2,
                problem_id: Some("P1001".to_string()),
                submitter: "bob".to_string(),
                verdict: "WA".to_string(),
                score: Some(60),
                time_ms: Some(48),
                memory_mb: Some(1.1),
                submitted_at: None,
            },
        ]
    }

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

    fn sample_history() -> Vec<HistoricalSolveRecord> {
        vec![
            HistoricalSolveRecord {
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
            },
            HistoricalSolveRecord {
                revision: "fedcba6543210000".to_string(),
                record: crate::models::SolveRecord {
                    problem_id: "P1001".to_string(),
                    title: "A+B Problem".to_string(),
                    verdict: "AC".to_string(),
                    score: None,
                    time_ms: None,
                    memory_mb: None,
                    difficulty: "入门".to_string(),
                    tags: vec!["模拟".to_string()],
                    source: "Luogu".to_string(),
                    submission_id: Some(2),
                    submission_time: None,
                    file_name: "P1001.cpp".to_string(),
                    training: TrainingFields::default(),
                    source_order: 0,
                },
            },
        ]
    }

    #[test]
    fn initial_selection_is_first_item_when_submissions_exist() {
        let submissions = sample_submissions();

        assert_eq!(initial_selection(&submissions), Some(0));
        assert_eq!(initial_selection(&[]), None);
        assert_eq!(initial_selection_for_records(&sample_history()), Some(0));
        assert_eq!(initial_selection_for_records(&[]), None);
    }

    #[test]
    fn apply_key_code_handles_submission_navigation_and_selection() {
        let submissions = sample_submissions();

        match apply_submission_key_code(
            KeyCode::Down,
            Some(0),
            &submissions,
            SubmissionSelectorMode::Sync,
        ) {
            SelectionOutcome::Continue(next) => assert_eq!(next, Some(1)),
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_submission_key_code(
            KeyCode::Enter,
            Some(1),
            &submissions,
            SubmissionSelectorMode::Sync,
        ) {
            SelectionOutcome::Select(crate::models::SyncSelection::Submission(record)) => {
                assert_eq!(record.submission_id, 2);
            }
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_submission_key_code(
            KeyCode::Char('c'),
            Some(0),
            &submissions,
            SubmissionSelectorMode::Sync,
        ) {
            SelectionOutcome::Select(crate::models::SyncSelection::Chore) => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_submission_key_code(
            KeyCode::Esc,
            Some(0),
            &submissions,
            SubmissionSelectorMode::Sync,
        ) {
            SelectionOutcome::Select(crate::models::SyncSelection::Skip) => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }
    }

    #[test]
    fn apply_key_code_does_not_create_submission_when_no_records_exist() {
        match apply_submission_key_code(KeyCode::Enter, None, &[], SubmissionSelectorMode::Sync) {
            SelectionOutcome::Ignore => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_submission_key_code(KeyCode::Char('c'), None, &[], SubmissionSelectorMode::Sync)
        {
            SelectionOutcome::Select(crate::models::SyncSelection::Chore) => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_submission_key_code(KeyCode::Esc, None, &[], SubmissionSelectorMode::Sync) {
            SelectionOutcome::Select(crate::models::SyncSelection::Skip) => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }
    }

    #[test]
    fn record_submission_mode_skips_chore_shortcut() {
        let submissions = sample_submissions();

        match apply_submission_key_code(
            KeyCode::Char('c'),
            Some(0),
            &submissions,
            SubmissionSelectorMode::Record,
        ) {
            SelectionOutcome::Ignore => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        assert_eq!(
            submission_footer_text(SubmissionSelectorMode::Record),
            "↑/↓ 移动  Enter 确认  Esc 取消"
        );
        assert_eq!(
            submission_empty_state_text(SubmissionSelectorMode::Record),
            "未找到提交记录\n按 Esc 返回"
        );
    }

    #[test]
    fn record_selection_key_code_handles_navigation_and_cancel() {
        let records = sample_history();

        match apply_record_key_code(KeyCode::Down, Some(0), &records) {
            RecordSelectionOutcome::Continue(next) => assert_eq!(next, Some(1)),
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_record_key_code(KeyCode::Enter, Some(1), &records) {
            RecordSelectionOutcome::Select(index) => assert_eq!(index, Some(1)),
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_record_key_code(KeyCode::Esc, Some(0), &records) {
            RecordSelectionOutcome::Select(None) => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }
    }

    #[test]
    fn apply_delete_key_code_supports_delete_and_skip_only() {
        match apply_delete_key_code(KeyCode::Enter) {
            SelectionOutcome::Select(crate::models::SyncSelection::Delete) => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_delete_key_code(KeyCode::Esc) {
            SelectionOutcome::Select(crate::models::SyncSelection::Skip) => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }

        match apply_delete_key_code(KeyCode::Char('c')) {
            SelectionOutcome::Ignore => {}
            outcome => panic!("unexpected outcome: {outcome:?}"),
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
        let submissions = sample_submissions();
        let row = build_submission_row(&submissions[0]);
        let debug_row = format!("{row:?}");

        assert!(debug_row.contains("alice"));
        assert!(debug_row.contains("1"));
        assert!(debug_row.contains("AC"));
    }

    #[test]
    fn build_record_row_includes_revision_and_submission_id() {
        let row = build_record_row(&sample_history()[0]);
        let debug_row = format!("{row:?}");

        assert!(debug_row.contains("abcdef123456"));
        assert!(debug_row.contains("1"));
        assert!(debug_row.contains("WA"));
    }

    #[test]
    fn short_revision_truncates_long_revision_ids() {
        assert_eq!(short_revision("abcdef1234567890"), "abcdef123456");
    }

    #[test]
    fn verdict_style_colors_ac_and_wa_only() {
        assert_eq!(
            verdict_style("AC"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            verdict_style("WA"),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        );
        assert_eq!(verdict_style("TLE"), Style::default());
    }

    #[test]
    fn stats_helpers_render_header_overview_and_empty_state() {
        let summary = StatsSummary {
            total_solve_records: 3,
            unique_problem_count: 2,
            unique_ac_count: 1,
            unique_non_ac_count: 1,
            first_ac_count: 1,
            repeated_practice_count: 1,
            time_window_days: Some(7),
            verdict_counts: vec![("AC".to_string(), 2), ("WA".to_string(), 1)],
            difficulty_counts: vec![("入门".to_string(), 2)],
            tag_counts: vec![("模拟".to_string(), 2), ("二分".to_string(), 1)],
        };

        let header = format!(
            "{:?}",
            stats_header_lines(std::path::Path::new("/tmp/aclog"), &summary)
        );
        let overview = format!("{:?}", stats_overview_lines(&summary));
        let empty_state = format!("{:?}", stats_empty_state_lines());

        assert!(header.contains("/tmp/aclog"));
        assert!(header.contains("solve(...)"));
        assert!(header.contains("最近 7 天"));
        assert!(overview.contains("唯一题目数: 2"));
        assert!(overview.contains("solve 记录数: 3"));
        assert!(overview.contains("首次 AC: 1"));
        assert!(empty_state.contains("还没有已记录的做题提交"));
    }

    #[test]
    fn distribution_lines_render_items_and_empty_state() {
        let filled = format!(
            "{:?}",
            distribution_lines(&[("AC".to_string(), 2), ("WA".to_string(), 1)], "结果分布")
        );
        let empty = format!("{:?}", distribution_lines(&[], "结果分布"));

        assert!(filled.contains("AC: 2"));
        assert!(filled.contains("WA: 1"));
        assert!(empty.contains("无数据"));
    }

    #[test]
    fn should_exit_stats_supports_q_and_escape() {
        assert!(should_exit_stats(KeyCode::Char('q')));
        assert!(should_exit_stats(KeyCode::Esc));
        assert!(!should_exit_stats(KeyCode::Enter));
    }

    #[test]
    fn stats_content_mode_uses_empty_state_only_without_records() {
        let empty = StatsSummary {
            total_solve_records: 0,
            unique_problem_count: 0,
            unique_ac_count: 0,
            unique_non_ac_count: 0,
            first_ac_count: 0,
            repeated_practice_count: 0,
            time_window_days: None,
            verdict_counts: vec![],
            difficulty_counts: vec![],
            tag_counts: vec![],
        };
        let populated = StatsSummary {
            total_solve_records: 1,
            unique_problem_count: 1,
            unique_ac_count: 1,
            unique_non_ac_count: 0,
            first_ac_count: 1,
            repeated_practice_count: 0,
            time_window_days: None,
            verdict_counts: vec![("AC".to_string(), 1)],
            difficulty_counts: vec![("入门".to_string(), 1)],
            tag_counts: vec![("模拟".to_string(), 1)],
        };

        assert_eq!(stats_content_mode(&empty), StatsContentMode::Empty);
        assert_eq!(
            stats_content_mode(&populated),
            StatsContentMode::Distributions
        );
    }
}
