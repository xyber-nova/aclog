//! 训练统计工作台。
//!
//! 这里承载两种紧密相关的视图：
//! - Overview: 统计总览
//! - Review: 复习建议
//!
//! 它负责模式切换和 drill-down 到 browser，
//! 但不负责计算统计值或生成建议本身。

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
        browser::{BrowserQuery, BrowserRootView},
        record_index::RecordIndex,
        stats::{StatsDashboard, StatsSummary},
    },
    ui::terminal::{
        TerminalHandle,
        browser::run_browser_app,
        common::{
            clamp_selection, footer_panel, initial_selection_for_count, is_help, lines_panel,
            move_selection, root_vertical_layout, split_main_with_summary,
        },
        theme,
    },
    utils::normalize_verdict,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsMode {
    /// 统计概览页，展示各类聚合分布。
    Overview,
    /// 复习建议页，展示可继续钻取的候选项。
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsWorkflowAction {
    None,
    SwitchMode(StatsMode),
    Exit,
}

/// 只读统计模式入口。
///
/// 当调用方只有 `StatsSummary` 时，这里补一个空 review 列表，
/// 让外部仍然复用同一套 dashboard 工作台。
pub(crate) fn run_stats_app(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    summary: &StatsSummary,
) -> Result<()> {
    let dashboard = StatsDashboard {
        summary: summary.clone(),
        review_candidates: Vec::new(),
        start_in_review: false,
    };
    let empty_index = RecordIndex::build(&[]);
    run_stats_dashboard_app(terminal, workspace_root, &dashboard, &empty_index)
}

/// 完整统计工作台主循环。
pub(crate) fn run_stats_dashboard_app(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    dashboard: &StatsDashboard,
    index: &RecordIndex,
) -> Result<()> {
    let mut mode = if dashboard.start_in_review {
        StatsMode::Review
    } else {
        StatsMode::Overview
    };
    let mut review_selected = 0usize;
    let mut help_visible = false;

    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 4);
            let sections =
                ratatui::layout::Layout::vertical([Constraint::Length(8), Constraint::Min(8)])
                    .split(content);

            frame.render_widget(
                crate::ui::terminal::common::lines_panel(
                    "训练统计工作台",
                    stats_header_lines(workspace_root, &dashboard.summary, mode),
                ),
                header,
            );
            frame.render_widget(
                crate::ui::terminal::common::lines_panel(
                    "总体概览",
                    stats_overview_lines(&dashboard.summary),
                ),
                sections[0],
            );

            match mode {
                StatsMode::Overview => {
                    let chunks = ratatui::layout::Layout::horizontal([
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                        Constraint::Percentage(33),
                    ])
                    .split(sections[1]);
                    frame.render_widget(
                        crate::ui::terminal::common::lines_panel(
                            "结果分布",
                            distribution_lines(
                                &dashboard.summary.verdict_counts,
                                "当前工作区本地 solve 记录的结果分布",
                            ),
                        ),
                        chunks[0],
                    );
                    frame.render_widget(
                        crate::ui::terminal::common::lines_panel(
                            "难度分布",
                            distribution_lines(
                                &dashboard.summary.difficulty_counts,
                                "按题号去重后的最新记录难度分布",
                            ),
                        ),
                        chunks[1],
                    );
                    frame.render_widget(
                        crate::ui::terminal::common::lines_panel(
                            "标签分布",
                            distribution_lines(
                                &dashboard.summary.tag_counts,
                                "按题号去重后的最新记录算法标签分布",
                            ),
                        ),
                        chunks[2],
                    );
                }
                StatsMode::Review => {
                    let [list_area, detail_area] = split_main_with_summary(sections[1], 44, 56);
                    let rows = dashboard
                        .review_candidates
                        .iter()
                        .map(|item| {
                            Row::new([
                                Cell::from(item.kind.clone()),
                                Cell::from(item.label.clone()),
                                Cell::from(
                                    item.verdict
                                        .as_deref()
                                        .map(|value| normalize_verdict(value).into_owned())
                                        .unwrap_or_else(|| "-".to_string()),
                                )
                                .style(theme::verdict_style(
                                    item.verdict.as_deref().unwrap_or("-"),
                                )),
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
                            .style(theme::accent_style().add_modifier(Modifier::BOLD)),
                    )
                    .block(crate::ui::terminal::common::panel("复习建议"))
                    .row_highlight_style(theme::selected_row_style())
                    .highlight_symbol(theme::FOCUS_SYMBOL);
                    let mut state = TableState::default().with_selected(
                        initial_selection_for_count(dashboard.review_candidates.len()).map(|_| {
                            clamp_selection(review_selected, dashboard.review_candidates.len())
                        }),
                    );
                    frame.render_stateful_widget(table, list_area, &mut state);

                    let detail = dashboard
                        .review_candidates
                        .get(clamp_selection(
                            review_selected,
                            dashboard.review_candidates.len(),
                        ))
                        .map(review_detail_lines)
                        .unwrap_or_else(|| vec![Line::from("当前没有可用的复习建议")]);
                    frame.render_widget(lines_panel("建议详情", detail), detail_area);
                }
            }

            let core = match mode {
                StatsMode::Overview => {
                    "Tab 切换模式  f 文件浏览  p 题目浏览  Esc 返回/退出  q 退出"
                }
                StatsMode::Review => {
                    "j/k/↑/↓ 移动  Tab 切换模式  Enter 打开历史  Esc 返回概览  q 退出"
                }
            };
            let help_lines = match mode {
                StatsMode::Overview => &[
                    "Tab: 在 overview / review 间切换",
                    "f: 打开文件浏览工作台",
                    "p: 打开题目浏览工作台",
                    "Esc: 在 overview 中退出工作台",
                    "q: 直接退出工作台",
                ][..],
                StatsMode::Review => &[
                    "Tab: 切回 overview",
                    "Enter: 打开当前建议对应的历史",
                    "Esc: 返回 overview",
                    "b: 兼容返回别名",
                    "q: 直接退出工作台",
                ][..],
            };
            frame.render_widget(footer_panel(core, help_lines, help_visible), footer);
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
        let workflow_action = stats_workflow_action(mode, key.code);
        if apply_stats_workflow_action(&mut mode, &mut review_selected, workflow_action) {
            return Ok(());
        }
        match mode {
            StatsMode::Overview => match key.code {
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
                _ => {}
            },
            StatsMode::Review => match key.code {
                code if move_selection(
                    code,
                    Some(review_selected),
                    dashboard.review_candidates.len(),
                )
                .is_some() =>
                {
                    if let Some(Some(next)) = move_selection(
                        code,
                        Some(review_selected),
                        dashboard.review_candidates.len(),
                    ) {
                        review_selected = next;
                    }
                }
                KeyCode::Enter => {
                    if let Some(candidate) = dashboard.review_candidates.get(clamp_selection(
                        review_selected,
                        dashboard.review_candidates.len(),
                    )) {
                        let query = if let Some(problem_id) = candidate.problem_id.as_ref() {
                            BrowserQuery {
                                root_view: BrowserRootView::Problems,
                                problem_id: Some(problem_id.clone()),
                                timeline_problem: Some(problem_id.clone()),
                                return_to_caller_on_escape: true,
                                ..BrowserQuery::default()
                            }
                        } else {
                            BrowserQuery {
                                root_view: BrowserRootView::Problems,
                                tag: Some(candidate.label.clone()),
                                return_to_caller_on_escape: true,
                                ..BrowserQuery::default()
                            }
                        };
                        run_browser_app(terminal, workspace_root, &query, index)?;
                    }
                }
                _ => {}
            },
        }
    }
}

fn stats_workflow_action(mode: StatsMode, key: KeyCode) -> StatsWorkflowAction {
    match mode {
        StatsMode::Overview => match key {
            KeyCode::Tab | KeyCode::Char('r') => StatsWorkflowAction::SwitchMode(StatsMode::Review),
            KeyCode::Esc | KeyCode::Char('q') => StatsWorkflowAction::Exit,
            _ => StatsWorkflowAction::None,
        },
        StatsMode::Review => match key {
            KeyCode::Tab | KeyCode::Esc | KeyCode::Char('b') => {
                StatsWorkflowAction::SwitchMode(StatsMode::Overview)
            }
            KeyCode::Char('q') => StatsWorkflowAction::Exit,
            _ => StatsWorkflowAction::None,
        },
    }
}

fn apply_stats_workflow_action(
    mode: &mut StatsMode,
    review_selected: &mut usize,
    action: StatsWorkflowAction,
) -> bool {
    match action {
        StatsWorkflowAction::None => false,
        StatsWorkflowAction::SwitchMode(next) => {
            *mode = next;
            *review_selected = 0;
            false
        }
        StatsWorkflowAction::Exit => true,
    }
}

/// 统计页头部摘要。
fn stats_header_lines(
    workspace_root: &std::path::Path,
    summary: &StatsSummary,
    mode: StatsMode,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("工作区: {}", workspace_root.display())),
        Line::from(match mode {
            StatsMode::Overview => "[概览模式] 使用 Tab 切换到 review 建议".to_string(),
            StatsMode::Review => "[建议模式] 使用 Tab 或 Esc 返回概览".to_string(),
        }),
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

/// 总览页顶部指标摘要。
fn stats_overview_lines(summary: &StatsSummary) -> Vec<Line<'static>> {
    vec![
        Line::from(format!("唯一题目数: {}", summary.unique_problem_count)),
        Line::from(format!("solve 记录数: {}", summary.total_solve_records)),
        Line::from(vec![
            Span::raw("唯一题目 "),
            Span::styled("AC", theme::verdict_style("AC")),
            Span::raw(format!(": {}", summary.unique_ac_count)),
        ]),
        Line::from(vec![
            Span::raw("唯一题目非 "),
            Span::styled("AC", theme::verdict_style("AC")),
            Span::raw(format!(": {}", summary.unique_non_ac_count)),
        ]),
        Line::from(vec![
            Span::raw("首次 "),
            Span::styled("AC", theme::verdict_style("AC")),
            Span::raw(format!(": {}", summary.first_ac_count)),
        ]),
        Line::from(format!("重复练习题数: {}", summary.repeated_practice_count)),
    ]
}

/// 把分布型统计渲染成简单的标签计数列表。
fn distribution_lines(items: &[(String, usize)], description: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(description.to_string()), Line::from("")];
    if items.is_empty() {
        lines.push(Line::from("无数据"));
        return lines;
    }
    lines.extend(items.iter().map(|(label, count)| {
        let normalized = normalize_verdict(label).into_owned();
        let style = theme::verdict_style(&normalized);
        if style == ratatui::style::Style::default() {
            Line::from(format!("{label}: {count}"))
        } else {
            Line::from(vec![
                Span::styled(normalized, style),
                Span::raw(format!(": {count}")),
            ])
        }
    }));
    lines
}

fn review_detail_lines(item: &crate::domain::stats::ReviewCandidate) -> Vec<Line<'static>> {
    let verdict = item
        .verdict
        .as_deref()
        .map(|value| normalize_verdict(value).into_owned())
        .unwrap_or_else(|| "-".to_string());

    vec![
        Line::from(format!("标签: {}", item.label)),
        Line::from(vec![
            Span::raw("结果: "),
            Span::styled(verdict.clone(), theme::verdict_style(&verdict)),
        ]),
        Line::from(format!("原因: {}", item.reason)),
        Line::from(format!(
            "上次时间: {}",
            item.last_submission_time
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "-".to_string()),
        )),
    ]
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::domain::stats::StatsSummary;

    use super::{
        StatsMode, StatsWorkflowAction, apply_stats_workflow_action, distribution_lines,
        stats_header_lines, stats_overview_lines, stats_workflow_action,
    };

    #[test]
    fn stats_helpers_render_header_and_overview() {
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
            stats_header_lines(
                std::path::Path::new("/tmp/aclog"),
                &summary,
                StatsMode::Review
            )
        );
        let overview = format!("{:?}", stats_overview_lines(&summary));
        let empty = format!("{:?}", distribution_lines(&[], "结果分布"));

        assert!(header.contains("[建议模式]"));
        assert!(header.contains("最近 7 天"));
        assert!(overview.contains("首次 "));
        assert!(overview.contains("\"AC\""));
        assert!(empty.contains("无数据"));
    }

    #[test]
    fn stats_workflow_maps_mode_switch_and_exit_keys() {
        assert_eq!(
            stats_workflow_action(StatsMode::Overview, KeyCode::Tab),
            StatsWorkflowAction::SwitchMode(StatsMode::Review)
        );
        assert_eq!(
            stats_workflow_action(StatsMode::Overview, KeyCode::Char('r')),
            StatsWorkflowAction::SwitchMode(StatsMode::Review)
        );
        assert_eq!(
            stats_workflow_action(StatsMode::Review, KeyCode::Esc),
            StatsWorkflowAction::SwitchMode(StatsMode::Overview)
        );
        assert_eq!(
            stats_workflow_action(StatsMode::Review, KeyCode::Char('q')),
            StatsWorkflowAction::Exit
        );
    }

    #[test]
    fn switching_stats_mode_resets_review_focus() {
        let mut mode = StatsMode::Review;
        let mut review_selected = 5usize;

        assert!(!apply_stats_workflow_action(
            &mut mode,
            &mut review_selected,
            StatsWorkflowAction::SwitchMode(StatsMode::Overview),
        ));
        assert_eq!(mode, StatsMode::Overview);
        assert_eq!(review_selected, 0);
    }
}
