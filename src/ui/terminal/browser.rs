//! 记录浏览工作台。
//!
//! 这一模块负责在“文件 / 题目 / 时间线”几个浏览视图之间切换，
//! 但不负责构建数据索引或解释过滤语义；那些能力都留在 domain 层。

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
        browser::{
            BrowserProviderView, BrowserQuery, BrowserRootView, build_browser_state,
            filter_browser_files, filter_browser_problems, filter_timeline_rows,
            timeline_rows_for_file, timeline_rows_for_problem,
        },
        record_index::RecordIndex,
    },
    problem::{human_problem_id, provider_label},
    ui::terminal::{
        TerminalHandle,
        common::{
            clamp_selection, footer_panel, initial_selection_for_count, is_help, lines_panel,
            move_selection, root_vertical_layout, split_main_with_summary,
        },
        theme,
    },
    utils::normalize_verdict,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum BrowserScreen {
    /// 根视图：按文件聚合的当前状态列表。
    Files,
    /// 根视图：按题目聚合的当前状态列表。
    Problems,
    // 时间线页沿用同一套工作台外壳，只是把左侧从“当前状态列表”
    // 切换成“完整 solve 历史时间线”。
    FileTimeline(String),
    ProblemTimeline(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BrowserWorkflowAction {
    None,
    SwitchProvider(BrowserProviderView),
    SwitchRoot(BrowserScreen),
    BackToRoot(BrowserScreen),
    Exit,
}

/// 记录浏览工作台主循环。
///
/// 它的职责是维护页面状态机、渲染当前 screen，并把按键翻译成视图跳转。
pub(crate) fn run_browser_app(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    query: &BrowserQuery,
    index: &RecordIndex,
) -> Result<()> {
    let state = build_browser_state(index);
    // stats / review 的钻取可以直接深链到某条时间线；
    // 否则就从 query 指定的根视角页签启动。
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
    let mut provider = query.provider;
    let mut selected = 0usize;
    let mut help_visible = false;

    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 4);
            let [list_area, detail_area] = split_main_with_summary(content, 46, 54);

            frame.render_widget(
                lines_panel(
                    "记录浏览工作台",
                    vec![
                        Line::from(format!("工作区: {}", workspace_root.display())),
                        Line::from(browser_header_summary(query, provider, &screen)),
                    ],
                ),
                header,
            );

            match &screen {
                BrowserScreen::Files => {
                    let rows =
                        filter_browser_files(&state.files, &query_with_provider(query, provider));
                    render_files_table(frame, list_area, &rows, selected);
                    let detail = rows
                        .get(clamp_selection(selected, rows.len()))
                        .and_then(|row| index.timeline_for_file(&row.file_name).first())
                        .map(render_record_detail_lines)
                        .unwrap_or_else(|| vec![Line::from("没有匹配记录")]);
                    frame.render_widget(lines_panel("详情", detail), detail_area);
                }
                BrowserScreen::Problems => {
                    let rows = filter_browser_problems(
                        &state.problems,
                        &query_with_provider(query, provider),
                    );
                    render_problems_table(frame, list_area, &rows, selected);
                    let detail = rows
                        .get(clamp_selection(selected, rows.len()))
                        .map(render_problem_detail_lines)
                        .unwrap_or_else(|| vec![Line::from("没有匹配记录")]);
                    frame.render_widget(lines_panel("详情", detail), detail_area);
                }
                BrowserScreen::FileTimeline(file_name) => {
                    let rows = filter_timeline_rows(
                        &timeline_rows_for_file(index, file_name),
                        &query_with_provider(query, provider),
                    );
                    render_file_timeline_table(frame, list_area, file_name, &rows, selected);
                    let detail = rows
                        .get(clamp_selection(selected, rows.len()))
                        .and_then(|row| {
                            index
                                .timeline_for_file(file_name)
                                .iter()
                                .find(|record| record.revision == row.revision)
                        })
                        .map(render_record_detail_lines)
                        .unwrap_or_else(|| vec![Line::from("没有匹配记录")]);
                    frame.render_widget(lines_panel("记录详情", detail), detail_area);
                }
                BrowserScreen::ProblemTimeline(problem_id) => {
                    let rows = filter_timeline_rows(
                        &timeline_rows_for_problem(index, problem_id),
                        &query_with_provider(query, provider),
                    );
                    render_problem_timeline_table(frame, list_area, problem_id, &rows, selected);
                    let detail = rows
                        .get(clamp_selection(selected, rows.len()))
                        .and_then(|row| {
                            index
                                .timeline_for_problem(problem_id)
                                .iter()
                                .find(|record| record.revision == row.revision)
                        })
                        .map(render_record_detail_lines)
                        .unwrap_or_else(|| vec![Line::from("没有匹配记录")]);
                    frame.render_widget(lines_panel("记录详情", detail), detail_area);
                }
            }

            let footer_text = match screen {
                BrowserScreen::Files | BrowserScreen::Problems => {
                    "j/k/↑/↓ 移动  Tab 切 Provider  f/p 切视角  Enter 打开时间线  Esc 退出  q 退出"
                }
                BrowserScreen::FileTimeline(_) | BrowserScreen::ProblemTimeline(_) => {
                    "j/k/↑/↓ 移动  Esc 返回  q 退出"
                }
            };
            let help_lines = match screen {
                BrowserScreen::Files | BrowserScreen::Problems => &[
                    "Tab: 在 Luogu / AtCoder / All 页签间切换",
                    "f / p: 切换文件/题目视角",
                    "Enter: 打开当前项时间线",
                    "Esc: 在根视图退出工作台",
                    "q: 直接退出工作台",
                ][..],
                BrowserScreen::FileTimeline(_) | BrowserScreen::ProblemTimeline(_) => {
                    &["Esc: 返回根视图", "b: 兼容返回别名", "q: 退出工作台"][..]
                }
            };
            frame.render_widget(footer_panel(footer_text, help_lines, help_visible), footer);
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
        let workflow_action = browser_workflow_action(&screen, provider, query, key.code);
        if apply_browser_workflow_action(&mut screen, &mut provider, &mut selected, workflow_action)
        {
            return Ok(());
        }

        match &mut screen {
            BrowserScreen::Files => match key.code {
                code if move_selection(
                    code,
                    Some(selected),
                    filter_browser_files(&state.files, &query_with_provider(query, provider)).len(),
                )
                .is_some() =>
                {
                    if let Some(Some(next)) = move_selection(
                        code,
                        Some(selected),
                        filter_browser_files(&state.files, &query_with_provider(query, provider))
                            .len(),
                    ) {
                        selected = next;
                    }
                }
                KeyCode::Enter => {
                    let rows =
                        filter_browser_files(&state.files, &query_with_provider(query, provider));
                    if let Some(row) = rows.get(clamp_selection(selected, rows.len())) {
                        // Enter 在这里统一表示“进入当前焦点对象的下一层”。
                        screen = BrowserScreen::FileTimeline(row.file_name.clone());
                        selected = 0;
                    }
                }
                _ => {}
            },
            BrowserScreen::Problems => match key.code {
                code if move_selection(
                    code,
                    Some(selected),
                    filter_browser_problems(&state.problems, &query_with_provider(query, provider))
                        .len(),
                )
                .is_some() =>
                {
                    if let Some(Some(next)) = move_selection(
                        code,
                        Some(selected),
                        filter_browser_problems(
                            &state.problems,
                            &query_with_provider(query, provider),
                        )
                        .len(),
                    ) {
                        selected = next;
                    }
                }
                KeyCode::Enter => {
                    let rows = filter_browser_problems(
                        &state.problems,
                        &query_with_provider(query, provider),
                    );
                    if let Some(row) = rows.get(clamp_selection(selected, rows.len())) {
                        screen = BrowserScreen::ProblemTimeline(row.problem_id.clone());
                        selected = 0;
                    }
                }
                _ => {}
            },
            BrowserScreen::FileTimeline(_) | BrowserScreen::ProblemTimeline(_) => match key.code {
                code if move_selection(code, Some(selected), usize::MAX).is_some() => {
                    if let Some(Some(next)) = move_selection(code, Some(selected), usize::MAX) {
                        selected = next;
                    }
                }
                _ => {}
            },
        }
    }
}

fn render_record_detail_lines(
    record: &crate::domain::record::HistoricalSolveRecord,
) -> Vec<Line<'static>> {
    let verdict = normalize_verdict(&record.record.verdict).into_owned();
    vec![
        Line::from(format!("版本: {}", record.revision)),
        Line::from(format!(
            "题号: {}",
            human_problem_id(&record.record.problem_id)
        )),
        Line::from(format!("标题: {}", record.record.title)),
        Line::from(format!("文件: {}", record.record.file_name)),
        Line::from(vec![
            Span::raw("结果: "),
            Span::styled(verdict.clone(), theme::verdict_style(&verdict)),
        ]),
        Line::from(format!(
            "分数: {}",
            record
                .record
                .score
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!(
            "耗时: {}",
            record
                .record
                .time_ms
                .map(|value| format!("{value}ms"))
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!(
            "内存: {}",
            record
                .record
                .memory_mb
                .map(|value| format!("{value:.1}MB"))
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!("难度: {}", record.record.difficulty)),
        Line::from(format!("来源: {}", provider_label(record.record.provider))),
        Line::from(format!(
            "比赛: {}",
            record.record.contest.as_deref().unwrap_or("-")
        )),
        Line::from(format!(
            "提交编号: {}",
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
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "-".to_string()),
        )),
        Line::from(format!(
            "标签: {}",
            if record.record.tags.is_empty() {
                "-".to_string()
            } else {
                record.record.tags.join(", ")
            }
        )),
        Line::from(format!(
            "笔记: {}",
            record.record.training.note.as_deref().unwrap_or("-")
        )),
        Line::from(format!(
            "卡点: {}",
            record.record.training.mistakes.as_deref().unwrap_or("-")
        )),
        Line::from(format!(
            "收获: {}",
            record.record.training.insight.as_deref().unwrap_or("-")
        )),
        Line::from(format!(
            "熟练度: {}",
            record.record.training.confidence.as_deref().unwrap_or("-")
        )),
        Line::from(format!(
            "完成方式: {}",
            record.record.training.source_kind.as_deref().unwrap_or("-")
        )),
        Line::from(format!(
            "训练耗时: {}",
            record.record.training.time_spent.as_deref().unwrap_or("-")
        )),
    ]
}

fn render_problem_detail_lines(
    row: &crate::domain::browser::BrowserProblemRow,
) -> Vec<Line<'static>> {
    let verdict = normalize_verdict(&row.verdict).into_owned();
    vec![
        Line::from(format!("题号: {}", human_problem_id(&row.problem_id))),
        Line::from(format!("标题: {}", row.title)),
        Line::from(format!("来源: {}", provider_label(row.provider))),
        Line::from(format!("比赛: {}", row.contest.as_deref().unwrap_or("-"))),
        Line::from(vec![
            Span::raw("结果: "),
            Span::styled(verdict.clone(), theme::verdict_style(&verdict)),
        ]),
        Line::from(format!("难度: {}", row.difficulty)),
        Line::from(format!(
            "文件: {}",
            if row.files.is_empty() {
                "-".to_string()
            } else {
                row.files.join(", ")
            }
        )),
        Line::from(format!(
            "标签: {}",
            if row.tags.is_empty() {
                "-".to_string()
            } else {
                row.tags.join(", ")
            }
        )),
        Line::from(format!("训练摘要: {}", row.training_summary)),
    ]
}

fn browser_workflow_action(
    screen: &BrowserScreen,
    provider: BrowserProviderView,
    query: &BrowserQuery,
    key: KeyCode,
) -> BrowserWorkflowAction {
    match screen {
        BrowserScreen::Files => match key {
            KeyCode::Tab => BrowserWorkflowAction::SwitchProvider(next_provider(provider)),
            KeyCode::Char('p') => BrowserWorkflowAction::SwitchRoot(BrowserScreen::Problems),
            KeyCode::Esc | KeyCode::Char('q') => BrowserWorkflowAction::Exit,
            _ => BrowserWorkflowAction::None,
        },
        BrowserScreen::Problems => match key {
            KeyCode::Tab => BrowserWorkflowAction::SwitchProvider(next_provider(provider)),
            KeyCode::Char('f') => BrowserWorkflowAction::SwitchRoot(BrowserScreen::Files),
            KeyCode::Esc | KeyCode::Char('q') => BrowserWorkflowAction::Exit,
            _ => BrowserWorkflowAction::None,
        },
        BrowserScreen::FileTimeline(_) | BrowserScreen::ProblemTimeline(_) => match key {
            KeyCode::Esc | KeyCode::Char('b') => {
                if query.return_to_caller_on_escape {
                    return BrowserWorkflowAction::Exit;
                }
                // `b` 只是兼容旧习惯；新的主语义是 `Esc` 返回上一层。
                BrowserWorkflowAction::BackToRoot(match query.root_view {
                    BrowserRootView::Files => BrowserScreen::Files,
                    BrowserRootView::Problems => BrowserScreen::Problems,
                })
            }
            KeyCode::Char('q') => BrowserWorkflowAction::Exit,
            _ => BrowserWorkflowAction::None,
        },
    }
}

fn apply_browser_workflow_action(
    screen: &mut BrowserScreen,
    provider: &mut BrowserProviderView,
    selected: &mut usize,
    action: BrowserWorkflowAction,
) -> bool {
    match action {
        BrowserWorkflowAction::None => false,
        BrowserWorkflowAction::SwitchProvider(next) => {
            *provider = next;
            *selected = 0;
            false
        }
        BrowserWorkflowAction::SwitchRoot(next) | BrowserWorkflowAction::BackToRoot(next) => {
            *screen = next;
            *selected = 0;
            false
        }
        BrowserWorkflowAction::Exit => true,
    }
}

/// 头部摘要文案，统一展示当前视角和过滤摘要。
fn browser_header_summary(
    query: &BrowserQuery,
    provider: BrowserProviderView,
    screen: &BrowserScreen,
) -> String {
    let mode = match screen {
        BrowserScreen::Files => "[文件视角]",
        BrowserScreen::Problems => "[题目视角]",
        BrowserScreen::FileTimeline(file) => {
            return format!(
                "[{}][文件时间线] {file}  {}",
                provider_summary(provider),
                browser_query_summary(query)
            );
        }
        BrowserScreen::ProblemTimeline(problem) => {
            return format!(
                "[{}][题目时间线] {}  {}",
                provider_summary(provider),
                human_problem_id(problem),
                browser_query_summary(query)
            );
        }
    };
    format!(
        "[{}]{mode}  {}",
        provider_summary(provider),
        browser_query_summary(query)
    )
}

/// 把 query 中的过滤条件压缩成适合头部扫读的一行说明。
fn browser_query_summary(query: &BrowserQuery) -> String {
    let mut parts = vec![match query.root_view {
        BrowserRootView::Files => "根视图=files".to_string(),
        BrowserRootView::Problems => "根视图=problems".to_string(),
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

fn provider_summary(provider: BrowserProviderView) -> &'static str {
    match provider {
        BrowserProviderView::Luogu => "Luogu",
        BrowserProviderView::AtCoder => "AtCoder",
        BrowserProviderView::All => "All",
    }
}

fn next_provider(provider: BrowserProviderView) -> BrowserProviderView {
    match provider {
        BrowserProviderView::Luogu => BrowserProviderView::AtCoder,
        BrowserProviderView::AtCoder => BrowserProviderView::All,
        BrowserProviderView::All => BrowserProviderView::Luogu,
    }
}

fn query_with_provider(query: &BrowserQuery, provider: BrowserProviderView) -> BrowserQuery {
    let mut updated = query.clone();
    updated.provider = provider;
    updated
}

/// 渲染文件根视图表格。
///
/// 数据过滤和排序已经在 domain 层完成，这里只负责呈现。
fn render_files_table(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    rows: &[crate::domain::browser::BrowserFileRow],
    selected: usize,
) {
    let table = Table::new(
        rows.iter()
            .map(|row| {
                Row::new([
                    Cell::from(row.file_name.clone()),
                    Cell::from(human_problem_id(&row.problem_id)),
                    Cell::from(normalize_verdict(&row.verdict).into_owned())
                        .style(theme::verdict_style(&row.verdict)),
                    Cell::from(
                        row.submission_time
                            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                ])
            })
            .collect::<Vec<_>>(),
        [
            Constraint::Percentage(44),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(16),
        ],
    )
    .header(
        Row::new(["文件", "题号", "结果", "记录时间"])
            .style(theme::accent_style().add_modifier(Modifier::BOLD)),
    )
    .block(crate::ui::terminal::common::panel("文件视角"))
    .row_highlight_style(theme::selected_row_style())
    .highlight_symbol(theme::FOCUS_SYMBOL);
    let mut state = TableState::default().with_selected(
        initial_selection_for_count(rows.len()).map(|_| clamp_selection(selected, rows.len())),
    );
    frame.render_stateful_widget(table, area, &mut state);
}

/// 渲染题目根视图表格。
fn render_problems_table(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    rows: &[crate::domain::browser::BrowserProblemRow],
    selected: usize,
) {
    let table = Table::new(
        rows.iter()
            .map(|row| {
                Row::new([
                    Cell::from(human_problem_id(&row.problem_id)),
                    Cell::from(normalize_verdict(&row.verdict).into_owned())
                        .style(theme::verdict_style(&row.verdict)),
                    Cell::from(row.files.len().to_string()),
                    Cell::from(
                        row.submission_time
                            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                ])
            })
            .collect::<Vec<_>>(),
        [
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(16),
        ],
    )
    .header(
        Row::new(["题号", "结果", "文件数", "记录时间"])
            .style(theme::accent_style().add_modifier(Modifier::BOLD)),
    )
    .block(crate::ui::terminal::common::panel("题目视角"))
    .row_highlight_style(theme::selected_row_style())
    .highlight_symbol(theme::FOCUS_SYMBOL);
    let mut state = TableState::default().with_selected(
        initial_selection_for_count(rows.len()).map(|_| clamp_selection(selected, rows.len())),
    );
    frame.render_stateful_widget(table, area, &mut state);
}

/// 渲染文件时间线表格。
fn render_file_timeline_table(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    file_name: &str,
    rows: &[crate::domain::browser::BrowserTimelineRow],
    selected: usize,
) {
    let table = Table::new(
        rows.iter()
            .map(|row| {
                Row::new([
                    Cell::from(
                        row.submission_time
                            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    Cell::from(normalize_verdict(&row.verdict).into_owned())
                        .style(theme::verdict_style(&row.verdict)),
                    Cell::from(row.revision.chars().take(12).collect::<String>()),
                ])
            })
            .collect::<Vec<_>>(),
        [
            Constraint::Length(16),
            Constraint::Length(8),
            Constraint::Length(14),
        ],
    )
    .header(
        Row::new(["提交时间", "结果", "Revision"])
            .style(theme::accent_style().add_modifier(Modifier::BOLD)),
    )
    .block(crate::ui::terminal::common::panel(format!(
        "文件时间线: {file_name}"
    )))
    .row_highlight_style(theme::selected_row_style())
    .highlight_symbol(theme::FOCUS_SYMBOL);
    let mut state = TableState::default().with_selected(
        initial_selection_for_count(rows.len()).map(|_| clamp_selection(selected, rows.len())),
    );
    frame.render_stateful_widget(table, area, &mut state);
}

/// 渲染题目时间线表格。
fn render_problem_timeline_table(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    problem_id: &str,
    rows: &[crate::domain::browser::BrowserTimelineRow],
    selected: usize,
) {
    let table = Table::new(
        rows.iter()
            .map(|row| {
                Row::new([
                    Cell::from(
                        row.submission_time
                            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    Cell::from(row.file_name.clone()),
                    Cell::from(normalize_verdict(&row.verdict).into_owned())
                        .style(theme::verdict_style(&row.verdict)),
                ])
            })
            .collect::<Vec<_>>(),
        [
            Constraint::Length(16),
            Constraint::Percentage(55),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new(["提交时间", "文件", "结果"])
            .style(theme::accent_style().add_modifier(Modifier::BOLD)),
    )
    .block(crate::ui::terminal::common::panel(format!(
        "题目时间线: {problem_id}"
    )))
    .row_highlight_style(theme::selected_row_style())
    .highlight_symbol(theme::FOCUS_SYMBOL);
    let mut state = TableState::default().with_selected(
        initial_selection_for_count(rows.len()).map(|_| clamp_selection(selected, rows.len())),
    );
    frame.render_stateful_widget(table, area, &mut state);
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use super::{
        BrowserScreen, BrowserWorkflowAction, apply_browser_workflow_action, browser_query_summary,
        browser_workflow_action,
    };
    use crate::domain::browser::{BrowserProviderView, BrowserQuery, BrowserRootView};

    #[test]
    fn browser_query_summary_renders_filter_summary() {
        let summary = browser_query_summary(&BrowserQuery {
            provider: BrowserProviderView::All,
            root_view: BrowserRootView::Problems,
            problem_id: Some("P1001".to_string()),
            file_name: None,
            verdict: Some("WA".to_string()),
            difficulty: None,
            tag: Some("模拟".to_string()),
            days: Some(7),
            timeline_file: None,
            timeline_problem: None,
            return_to_caller_on_escape: false,
            json: false,
        });

        assert!(summary.contains("根视图=problems"));
        assert!(summary.contains("题号=P1001"));
        assert!(summary.contains("结果=WA"));
        assert!(summary.contains("标签=模拟"));
        assert!(summary.contains("最近 7 天"));
    }

    #[test]
    fn browser_workflow_switches_root_view_and_resets_focus() {
        let query = BrowserQuery::default();
        let action = browser_workflow_action(
            &BrowserScreen::Files,
            BrowserProviderView::Luogu,
            &query,
            KeyCode::Tab,
        );
        let mut provider = BrowserProviderView::Luogu;
        let mut screen = BrowserScreen::Files;
        let mut selected = 7usize;

        assert_eq!(
            action,
            BrowserWorkflowAction::SwitchProvider(BrowserProviderView::AtCoder)
        );
        assert!(!apply_browser_workflow_action(
            &mut screen,
            &mut provider,
            &mut selected,
            action
        ));
        assert_eq!(screen, BrowserScreen::Files);
        assert_eq!(provider, BrowserProviderView::AtCoder);
        assert_eq!(selected, 0);
    }

    #[test]
    fn browser_workflow_returns_to_query_root_from_timeline() {
        let query = BrowserQuery {
            root_view: BrowserRootView::Problems,
            return_to_caller_on_escape: false,
            ..BrowserQuery::default()
        };
        let action = browser_workflow_action(
            &BrowserScreen::FileTimeline("P1001.cpp".to_string()),
            BrowserProviderView::All,
            &query,
            KeyCode::Esc,
        );
        let mut provider = BrowserProviderView::All;
        let mut screen = BrowserScreen::FileTimeline("P1001.cpp".to_string());
        let mut selected = 3usize;

        assert_eq!(
            action,
            BrowserWorkflowAction::BackToRoot(BrowserScreen::Problems)
        );
        assert!(!apply_browser_workflow_action(
            &mut screen,
            &mut provider,
            &mut selected,
            action
        ));
        assert_eq!(screen, BrowserScreen::Problems);
        assert_eq!(selected, 0);
    }

    #[test]
    fn browser_workflow_allows_direct_quit() {
        let query = BrowserQuery::default();

        assert_eq!(
            browser_workflow_action(
                &BrowserScreen::Files,
                BrowserProviderView::All,
                &query,
                KeyCode::Char('q'),
            ),
            BrowserWorkflowAction::Exit
        );
        assert_eq!(
            browser_workflow_action(
                &BrowserScreen::ProblemTimeline("P1001".to_string()),
                BrowserProviderView::All,
                &query,
                KeyCode::Char('q'),
            ),
            BrowserWorkflowAction::Exit
        );
    }

    #[test]
    fn browser_timeline_escape_can_return_to_caller_for_drill_down_flow() {
        let query = BrowserQuery {
            root_view: BrowserRootView::Problems,
            timeline_problem: Some("P1001".to_string()),
            return_to_caller_on_escape: true,
            ..BrowserQuery::default()
        };

        assert_eq!(
            browser_workflow_action(
                &BrowserScreen::ProblemTimeline("P1001".to_string()),
                BrowserProviderView::All,
                &query,
                KeyCode::Esc,
            ),
            BrowserWorkflowAction::Exit
        );
    }
}
