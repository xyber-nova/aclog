//! 训练统计工作台。
//!
//! 这里承载三种紧密相关的视图：
//! - Overview: 统计总览
//! - ProblemReview: 题目复习
//! - TagPractice: 标签加练
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
        stats::{ProblemReviewCandidate, StatsDashboard, StatsSummary, TagPracticeSuggestion},
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
    Overview,
    ProblemReview,
    TagPractice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsWorkflowAction {
    None,
    SwitchMode(StatsMode),
    Exit,
}

pub(crate) fn run_stats_app(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    summary: &StatsSummary,
) -> Result<()> {
    let dashboard = StatsDashboard {
        summary: summary.clone(),
        problem_reviews: Vec::new(),
        tag_practice_suggestions: Vec::new(),
        start_in_review: false,
    };
    let empty_index = RecordIndex::build(&[]);
    run_stats_dashboard_app(terminal, workspace_root, &dashboard, &empty_index)
}

pub(crate) fn run_stats_dashboard_app(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    dashboard: &StatsDashboard,
    index: &RecordIndex,
) -> Result<()> {
    let mut mode = if dashboard.start_in_review {
        StatsMode::ProblemReview
    } else {
        StatsMode::Overview
    };
    let mut problem_selected = 0usize;
    let mut tag_selected = 0usize;
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
                StatsMode::ProblemReview => {
                    let [list_area, detail_area] = split_main_with_summary(sections[1], 44, 56);
                    let rows = dashboard
                        .problem_reviews
                        .iter()
                        .map(|item| {
                            Row::new([
                                Cell::from(item.problem_id.clone()),
                                Cell::from(item.verdict.clone())
                                    .style(theme::verdict_style(&item.verdict)),
                                Cell::from(item.priority.to_string()),
                            ])
                        })
                        .collect::<Vec<_>>();
                    let table = Table::new(
                        rows,
                        [
                            Constraint::Percentage(50),
                            Constraint::Length(10),
                            Constraint::Length(10),
                        ],
                    )
                    .header(
                        Row::new(["题号", "结果", "优先级"])
                            .style(theme::accent_style().add_modifier(Modifier::BOLD)),
                    )
                    .block(crate::ui::terminal::common::panel("题目复习"))
                    .row_highlight_style(theme::selected_row_style())
                    .highlight_symbol(theme::FOCUS_SYMBOL);
                    let mut state = TableState::default().with_selected(
                        initial_selection_for_count(dashboard.problem_reviews.len()).map(|_| {
                            clamp_selection(problem_selected, dashboard.problem_reviews.len())
                        }),
                    );
                    frame.render_stateful_widget(table, list_area, &mut state);

                    let detail = selected_problem_review_detail_lines(
                        &dashboard.problem_reviews,
                        problem_selected,
                    );
                    frame.render_widget(lines_panel("复习详情", detail), detail_area);
                }
                StatsMode::TagPractice => {
                    let [list_area, detail_area] = split_main_with_summary(sections[1], 44, 56);
                    let rows = dashboard
                        .tag_practice_suggestions
                        .iter()
                        .map(|item| {
                            Row::new([
                                Cell::from(item.tag.clone()),
                                Cell::from(item.recent_unique_problems.to_string()),
                                Cell::from(item.priority.to_string()),
                            ])
                        })
                        .collect::<Vec<_>>();
                    let table = Table::new(
                        rows,
                        [
                            Constraint::Percentage(50),
                            Constraint::Length(12),
                            Constraint::Length(10),
                        ],
                    )
                    .header(
                        Row::new(["标签", "最近题数", "建议分"])
                            .style(theme::accent_style().add_modifier(Modifier::BOLD)),
                    )
                    .block(crate::ui::terminal::common::panel("标签加练"))
                    .row_highlight_style(theme::selected_row_style())
                    .highlight_symbol(theme::FOCUS_SYMBOL);
                    let mut state = TableState::default().with_selected(
                        initial_selection_for_count(dashboard.tag_practice_suggestions.len()).map(
                            |_| {
                                clamp_selection(
                                    tag_selected,
                                    dashboard.tag_practice_suggestions.len(),
                                )
                            },
                        ),
                    );
                    frame.render_stateful_widget(table, list_area, &mut state);

                    let detail = selected_tag_practice_detail_lines(
                        &dashboard.tag_practice_suggestions,
                        tag_selected,
                    );
                    frame.render_widget(lines_panel("加练详情", detail), detail_area);
                }
            }

            let core = match mode {
                StatsMode::Overview => {
                    "Tab 切换模式  r 题目复习  f 文件浏览  p 题目浏览  Esc 返回/退出  q 退出"
                }
                StatsMode::ProblemReview => {
                    "j/k/↑/↓ 移动  Enter 打开题目历史  Tab 切到标签加练  Esc 返回概览  q 退出"
                }
                StatsMode::TagPractice => {
                    "j/k/↑/↓ 移动  Enter 打开标签题目  Tab 切回概览  Esc 返回概览  q 退出"
                }
            };
            let help_lines = match mode {
                StatsMode::Overview => &[
                    "Tab: 在 overview / 题目复习 / 标签加练 间循环切换",
                    "r: 直接进入题目复习",
                    "f: 打开文件浏览工作台",
                    "p: 打开题目浏览工作台",
                    "Esc: 在 overview 中退出工作台",
                    "q: 直接退出工作台",
                ][..],
                StatsMode::ProblemReview => &[
                    "Tab: 切到标签加练",
                    "Enter: 打开当前题目的时间线",
                    "Esc: 返回 overview",
                    "b: 兼容返回别名",
                    "f / p: 打开浏览工作台",
                    "q: 直接退出工作台",
                ][..],
                StatsMode::TagPractice => &[
                    "Tab: 切回 overview",
                    "Enter: 打开当前标签过滤后的题目视图",
                    "Esc: 返回 overview",
                    "b: 兼容返回别名",
                    "f / p: 打开浏览工作台",
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
        if apply_stats_workflow_action(
            &mut mode,
            &mut problem_selected,
            &mut tag_selected,
            workflow_action,
        ) {
            return Ok(());
        }
        if handle_global_browser_shortcuts(terminal, workspace_root, index, key.code)? {
            continue;
        }

        match mode {
            StatsMode::Overview => {}
            StatsMode::ProblemReview => match key.code {
                code if move_selection(
                    code,
                    Some(problem_selected),
                    dashboard.problem_reviews.len(),
                )
                .is_some() =>
                {
                    if let Some(Some(next)) = move_selection(
                        code,
                        Some(problem_selected),
                        dashboard.problem_reviews.len(),
                    ) {
                        problem_selected = next;
                    }
                }
                KeyCode::Enter => {
                    if let Some(candidate) = dashboard.problem_reviews.get(clamp_selection(
                        problem_selected,
                        dashboard.problem_reviews.len(),
                    )) {
                        run_browser_app(
                            terminal,
                            workspace_root,
                            &problem_review_browser_query(candidate),
                            index,
                        )?;
                    }
                }
                _ => {}
            },
            StatsMode::TagPractice => match key.code {
                code if move_selection(
                    code,
                    Some(tag_selected),
                    dashboard.tag_practice_suggestions.len(),
                )
                .is_some() =>
                {
                    if let Some(Some(next)) = move_selection(
                        code,
                        Some(tag_selected),
                        dashboard.tag_practice_suggestions.len(),
                    ) {
                        tag_selected = next;
                    }
                }
                KeyCode::Enter => {
                    if let Some(candidate) = dashboard.tag_practice_suggestions.get(
                        clamp_selection(tag_selected, dashboard.tag_practice_suggestions.len()),
                    ) {
                        run_browser_app(
                            terminal,
                            workspace_root,
                            &tag_practice_browser_query(candidate),
                            index,
                        )?;
                    }
                }
                _ => {}
            },
        }
    }
}

fn handle_global_browser_shortcuts(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    index: &RecordIndex,
    key: KeyCode,
) -> Result<bool> {
    match key {
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
            Ok(true)
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
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn stats_workflow_action(mode: StatsMode, key: KeyCode) -> StatsWorkflowAction {
    match mode {
        StatsMode::Overview => match key {
            KeyCode::Tab | KeyCode::Char('r') => {
                StatsWorkflowAction::SwitchMode(StatsMode::ProblemReview)
            }
            KeyCode::Esc | KeyCode::Char('q') => StatsWorkflowAction::Exit,
            _ => StatsWorkflowAction::None,
        },
        StatsMode::ProblemReview => match key {
            KeyCode::Tab => StatsWorkflowAction::SwitchMode(StatsMode::TagPractice),
            KeyCode::Esc | KeyCode::Char('b') => {
                StatsWorkflowAction::SwitchMode(StatsMode::Overview)
            }
            KeyCode::Char('q') => StatsWorkflowAction::Exit,
            _ => StatsWorkflowAction::None,
        },
        StatsMode::TagPractice => match key {
            KeyCode::Tab => StatsWorkflowAction::SwitchMode(StatsMode::Overview),
            KeyCode::Esc | KeyCode::Char('b') => {
                StatsWorkflowAction::SwitchMode(StatsMode::Overview)
            }
            KeyCode::Char('q') => StatsWorkflowAction::Exit,
            _ => StatsWorkflowAction::None,
        },
    }
}

fn apply_stats_workflow_action(
    mode: &mut StatsMode,
    problem_selected: &mut usize,
    tag_selected: &mut usize,
    action: StatsWorkflowAction,
) -> bool {
    match action {
        StatsWorkflowAction::None => false,
        StatsWorkflowAction::SwitchMode(next) => {
            *mode = next;
            *problem_selected = 0;
            *tag_selected = 0;
            false
        }
        StatsWorkflowAction::Exit => true,
    }
}

fn stats_header_lines(
    workspace_root: &std::path::Path,
    summary: &StatsSummary,
    mode: StatsMode,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("工作区: {}", workspace_root.display())),
        Line::from(match mode {
            StatsMode::Overview => "[概览模式] 使用 Tab 切换到题目复习 / 标签加练".to_string(),
            StatsMode::ProblemReview => {
                "[题目复习] 使用 Tab 切换到标签加练，Esc 返回概览".to_string()
            }
            StatsMode::TagPractice => "[标签加练] 使用 Tab 切回概览，Esc 返回概览".to_string(),
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

fn problem_review_detail_lines(item: &ProblemReviewCandidate) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("题号: {}", item.problem_id)),
        Line::from(format!("标题: {}", item.title)),
        Line::from(vec![
            Span::raw("结果: "),
            Span::styled(item.verdict.clone(), theme::verdict_style(&item.verdict)),
        ]),
        Line::from(format!("优先级: {}", item.priority)),
        Line::from(format!(
            "上次时间: {}",
            item.last_submission_time
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "-".to_string()),
        )),
    ];
    if item.matched_tags.is_empty() {
        lines.push(Line::from("算法标签: -"));
    } else {
        lines.push(Line::from(format!(
            "算法标签: {}",
            item.matched_tags.join(", ")
        )));
    }
    lines.push(Line::from("原因:"));
    lines.extend(
        item.reasons
            .iter()
            .map(|reason| Line::from(format!("- {reason}"))),
    );
    lines
}

fn selected_problem_review_detail_lines(
    items: &[ProblemReviewCandidate],
    selected: usize,
) -> Vec<Line<'static>> {
    items
        .get(clamp_selection(selected, items.len()))
        .map(problem_review_detail_lines)
        .unwrap_or_else(|| vec![Line::from("当前没有待复习题目")])
}

fn tag_practice_detail_lines(item: &TagPracticeSuggestion) -> Vec<Line<'static>> {
    vec![
        Line::from(format!("标签: {}", item.tag)),
        Line::from(format!("最近窗口题目数: {}", item.recent_unique_problems)),
        Line::from(format!("全历史题目数: {}", item.lifetime_unique_problems)),
        Line::from(format!(
            "最近不稳信号: {}",
            item.recent_unstable_signal_count
        )),
        Line::from(format!("建议分: {}", item.priority)),
        Line::from(format!("说明: {}", item.reason)),
    ]
}

fn selected_tag_practice_detail_lines(
    items: &[TagPracticeSuggestion],
    selected: usize,
) -> Vec<Line<'static>> {
    items
        .get(clamp_selection(selected, items.len()))
        .map(tag_practice_detail_lines)
        .unwrap_or_else(|| vec![Line::from("当前没有建议加练的标签")])
}

fn problem_review_browser_query(candidate: &ProblemReviewCandidate) -> BrowserQuery {
    BrowserQuery {
        root_view: BrowserRootView::Problems,
        problem_id: Some(candidate.problem_id.clone()),
        timeline_problem: Some(candidate.problem_id.clone()),
        return_to_caller_on_escape: true,
        ..BrowserQuery::default()
    }
}

fn tag_practice_browser_query(candidate: &TagPracticeSuggestion) -> BrowserQuery {
    BrowserQuery {
        root_view: BrowserRootView::Problems,
        tag: Some(candidate.tag.clone()),
        return_to_caller_on_escape: true,
        ..BrowserQuery::default()
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::domain::{
        browser::BrowserRootView,
        stats::{ProblemReviewCandidate, StatsSummary, TagPracticeSuggestion},
    };

    use super::{
        StatsMode, StatsWorkflowAction, apply_stats_workflow_action, distribution_lines,
        problem_review_browser_query, selected_problem_review_detail_lines,
        selected_tag_practice_detail_lines, stats_header_lines, stats_overview_lines,
        stats_workflow_action, tag_practice_browser_query,
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
                StatsMode::TagPractice
            )
        );
        let overview = format!("{:?}", stats_overview_lines(&summary));
        let empty = format!("{:?}", distribution_lines(&[], "结果分布"));

        assert!(header.contains("[标签加练]"));
        assert!(header.contains("最近 7 天"));
        assert!(overview.contains("首次 "));
        assert!(overview.contains("\"AC\""));
        assert!(empty.contains("无数据"));
    }

    #[test]
    fn stats_workflow_maps_mode_switch_and_exit_keys() {
        assert_eq!(
            stats_workflow_action(StatsMode::Overview, KeyCode::Tab),
            StatsWorkflowAction::SwitchMode(StatsMode::ProblemReview)
        );
        assert_eq!(
            stats_workflow_action(StatsMode::ProblemReview, KeyCode::Tab),
            StatsWorkflowAction::SwitchMode(StatsMode::TagPractice)
        );
        assert_eq!(
            stats_workflow_action(StatsMode::TagPractice, KeyCode::Tab),
            StatsWorkflowAction::SwitchMode(StatsMode::Overview)
        );
        assert_eq!(
            stats_workflow_action(StatsMode::ProblemReview, KeyCode::Esc),
            StatsWorkflowAction::SwitchMode(StatsMode::Overview)
        );
        assert_eq!(
            stats_workflow_action(StatsMode::TagPractice, KeyCode::Char('q')),
            StatsWorkflowAction::Exit
        );
    }

    #[test]
    fn switching_stats_mode_resets_review_focus() {
        let mut mode = StatsMode::TagPractice;
        let mut problem_selected = 5usize;
        let mut tag_selected = 7usize;

        assert!(!apply_stats_workflow_action(
            &mut mode,
            &mut problem_selected,
            &mut tag_selected,
            StatsWorkflowAction::SwitchMode(StatsMode::Overview),
        ));
        assert_eq!(mode, StatsMode::Overview);
        assert_eq!(problem_selected, 0);
        assert_eq!(tag_selected, 0);
    }

    #[test]
    fn browser_queries_match_problem_review_and_tag_practice_drill_down() {
        let problem_query = problem_review_browser_query(&ProblemReviewCandidate {
            problem_id: "P1001".to_string(),
            title: "A".to_string(),
            verdict: "WA".to_string(),
            last_submission_time: None,
            priority: 6,
            reasons: vec!["最近状态仍为 WA".to_string()],
            matched_tags: vec!["模拟".to_string()],
        });
        assert_eq!(problem_query.root_view, BrowserRootView::Problems);
        assert_eq!(problem_query.problem_id.as_deref(), Some("P1001"));
        assert_eq!(problem_query.timeline_problem.as_deref(), Some("P1001"));
        assert!(problem_query.return_to_caller_on_escape);

        let tag_query = tag_practice_browser_query(&TagPracticeSuggestion {
            tag: "二分".to_string(),
            recent_unique_problems: 1,
            lifetime_unique_problems: 1,
            priority: 399,
            reason: "最近 60 天仅练过 1 题，建议补样本".to_string(),
            recent_unstable_signal_count: 0,
        });
        assert_eq!(tag_query.root_view, BrowserRootView::Problems);
        assert_eq!(tag_query.tag.as_deref(), Some("二分"));
        assert_eq!(tag_query.timeline_problem, None);
        assert!(tag_query.return_to_caller_on_escape);
    }

    #[test]
    fn selected_detail_helpers_return_empty_states() {
        let problem_empty = format!("{:?}", selected_problem_review_detail_lines(&[], 0));
        let tag_empty = format!("{:?}", selected_tag_practice_detail_lines(&[], 0));

        assert!(problem_empty.contains("当前没有待复习题目"));
        assert!(tag_empty.contains("当前没有建议加练的标签"));
    }
}
