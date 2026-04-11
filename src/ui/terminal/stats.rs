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
        browser::{BrowserProviderView, BrowserQuery, BrowserRootView},
        record_index::RecordIndex,
        stats::{
            ProblemReviewCandidate, StatsDashboard, StatsProviderDashboard, StatsSummary,
            TagPracticeSuggestion,
        },
    },
    problem::human_problem_id,
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
    SwitchProvider(BrowserProviderView),
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
        provider_dashboards: fallback_provider_dashboards(summary),
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
    let fallback_dashboards = fallback_provider_dashboards(&dashboard.summary);
    let mut mode = if dashboard.start_in_review {
        StatsMode::ProblemReview
    } else {
        StatsMode::Overview
    };
    let mut provider = default_provider(dashboard);
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
                    stats_header_lines(
                        workspace_root,
                        &current_stats_dashboard(dashboard, &fallback_dashboards, provider).summary,
                        provider,
                        mode,
                    ),
                ),
                header,
            );
            let current = current_stats_dashboard(dashboard, &fallback_dashboards, provider);
            frame.render_widget(
                crate::ui::terminal::common::lines_panel(
                    "总体概览",
                    stats_overview_lines(&current.summary),
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
                                &current.summary.verdict_counts,
                                "当前工作区本地 solve 记录的结果分布",
                            ),
                        ),
                        chunks[0],
                    );
                    frame.render_widget(
                        crate::ui::terminal::common::lines_panel(
                            "难度分布",
                            distribution_lines(
                                &current.summary.difficulty_counts,
                                "按题号去重后的最新记录难度分布",
                            ),
                        ),
                        chunks[1],
                    );
                    frame.render_widget(
                        crate::ui::terminal::common::lines_panel(
                            if current.tag_features_supported {
                                "标签分布"
                            } else {
                                "标签分布（降级）"
                            },
                            if current.tag_features_supported {
                                distribution_lines(
                                    &current.summary.tag_counts,
                                    "按题号去重后的最新记录算法标签分布",
                                )
                            } else {
                                vec![
                                    Line::from("当前 provider 不支持标签统计"),
                                    Line::from("请切换到 Luogu 页签查看算法标签口径"),
                                ]
                            },
                        ),
                        chunks[2],
                    );
                }
                StatsMode::ProblemReview => {
                    let [list_area, detail_area] = split_main_with_summary(sections[1], 44, 56);
                    let rows = current
                        .problem_reviews
                        .iter()
                        .map(|item| {
                            Row::new([
                                Cell::from(human_problem_id(&item.problem_id)),
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
                        initial_selection_for_count(current.problem_reviews.len())
                            .map(|_| clamp_selection(problem_selected, current.problem_reviews.len())),
                    );
                    frame.render_stateful_widget(table, list_area, &mut state);

                    let detail =
                        selected_problem_review_detail_lines(&current.problem_reviews, problem_selected);
                    frame.render_widget(lines_panel("复习详情", detail), detail_area);
                }
                StatsMode::TagPractice => {
                    let [list_area, detail_area] = split_main_with_summary(sections[1], 44, 56);
                    if current.tag_features_supported {
                        let rows = current
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
                            initial_selection_for_count(current.tag_practice_suggestions.len())
                                .map(|_| {
                                    clamp_selection(
                                        tag_selected,
                                        current.tag_practice_suggestions.len(),
                                    )
                                }),
                        );
                        frame.render_stateful_widget(table, list_area, &mut state);

                        let detail = selected_tag_practice_detail_lines(
                            &current.tag_practice_suggestions,
                            tag_selected,
                        );
                        frame.render_widget(lines_panel("加练详情", detail), detail_area);
                    } else {
                        frame.render_widget(
                            lines_panel(
                                "标签加练（降级）",
                                vec![
                                    Line::from("当前 provider 不支持标签加练"),
                                    Line::from("请切换到 Luogu 页签使用标签分布与标签加练"),
                                ],
                            ),
                            list_area,
                        );
                        frame.render_widget(
                            lines_panel("加练详情", vec![Line::from("当前页签未提供标签建议")]),
                            detail_area,
                        );
                    }
                }
            }

            let core = match mode {
                StatsMode::Overview => {
                    "Tab 切 Provider  o/r/g 切模式  f 文件浏览  p 题目浏览  Esc 返回/退出  q 退出"
                }
                StatsMode::ProblemReview => {
                    "j/k/↑/↓ 移动  Enter 打开题目历史  Tab 切 Provider  o/r/g 切模式  Esc 返回概览  q 退出"
                }
                StatsMode::TagPractice => {
                    "j/k/↑/↓ 移动  Enter 打开标签题目  Tab 切 Provider  o/r/g 切模式  Esc 返回概览  q 退出"
                }
            };
            let help_lines = match mode {
                StatsMode::Overview => &[
                    "Tab: 在 Luogu / AtCoder / All 页签间循环切换",
                    "o / r / g: 切换概览 / 题目复习 / 标签加练",
                    "f: 打开文件浏览工作台",
                    "p: 打开题目浏览工作台",
                    "Esc: 在 overview 中退出工作台",
                    "q: 直接退出工作台",
                ][..],
                StatsMode::ProblemReview => &[
                    "Tab: 切换 provider 页签",
                    "Enter: 打开当前题目的时间线",
                    "Esc: 返回 overview",
                    "b: 兼容返回别名",
                    "f / p: 打开浏览工作台",
                    "q: 直接退出工作台",
                ][..],
                StatsMode::TagPractice => &[
                    "Tab: 切换 provider 页签",
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
        let workflow_action = stats_workflow_action(mode, provider, key.code);
        if apply_stats_workflow_action(
            &mut mode,
            &mut provider,
            &mut problem_selected,
            &mut tag_selected,
            workflow_action,
        ) {
            return Ok(());
        }
        if handle_global_browser_shortcuts(terminal, workspace_root, index, provider, key.code)? {
            continue;
        }

        match mode {
            StatsMode::Overview => {}
            StatsMode::ProblemReview => match key.code {
                code if move_selection(
                    code,
                    Some(problem_selected),
                    current_stats_dashboard(dashboard, &fallback_dashboards, provider)
                        .problem_reviews
                        .len(),
                )
                .is_some() =>
                {
                    if let Some(Some(next)) = move_selection(
                        code,
                        Some(problem_selected),
                        current_stats_dashboard(dashboard, &fallback_dashboards, provider)
                            .problem_reviews
                            .len(),
                    ) {
                        problem_selected = next;
                    }
                }
                KeyCode::Enter => {
                    let current =
                        current_stats_dashboard(dashboard, &fallback_dashboards, provider);
                    if let Some(candidate) = current.problem_reviews.get(clamp_selection(
                        problem_selected,
                        current.problem_reviews.len(),
                    )) {
                        run_browser_app(
                            terminal,
                            workspace_root,
                            &problem_review_browser_query(candidate, provider),
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
                    current_stats_dashboard(dashboard, &fallback_dashboards, provider)
                        .tag_practice_suggestions
                        .len(),
                )
                .is_some() =>
                {
                    if let Some(Some(next)) = move_selection(
                        code,
                        Some(tag_selected),
                        current_stats_dashboard(dashboard, &fallback_dashboards, provider)
                            .tag_practice_suggestions
                            .len(),
                    ) {
                        tag_selected = next;
                    }
                }
                KeyCode::Enter => {
                    let current =
                        current_stats_dashboard(dashboard, &fallback_dashboards, provider);
                    if let Some(candidate) = current.tag_practice_suggestions.get(clamp_selection(
                        tag_selected,
                        current.tag_practice_suggestions.len(),
                    )) {
                        run_browser_app(
                            terminal,
                            workspace_root,
                            &tag_practice_browser_query(candidate, provider),
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
    provider: BrowserProviderView,
    key: KeyCode,
) -> Result<bool> {
    match key {
        KeyCode::Char('f') => {
            run_browser_app(
                terminal,
                workspace_root,
                &BrowserQuery {
                    provider,
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
                    provider,
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

fn stats_workflow_action(
    mode: StatsMode,
    provider: BrowserProviderView,
    key: KeyCode,
) -> StatsWorkflowAction {
    match mode {
        StatsMode::Overview => match key {
            KeyCode::Tab => StatsWorkflowAction::SwitchProvider(next_provider(provider)),
            KeyCode::Char('o') => StatsWorkflowAction::SwitchMode(StatsMode::Overview),
            KeyCode::Char('r') => StatsWorkflowAction::SwitchMode(StatsMode::ProblemReview),
            KeyCode::Char('g') => StatsWorkflowAction::SwitchMode(StatsMode::TagPractice),
            KeyCode::Esc | KeyCode::Char('q') => StatsWorkflowAction::Exit,
            _ => StatsWorkflowAction::None,
        },
        StatsMode::ProblemReview => match key {
            KeyCode::Tab => StatsWorkflowAction::SwitchProvider(next_provider(provider)),
            KeyCode::Char('o') => StatsWorkflowAction::SwitchMode(StatsMode::Overview),
            KeyCode::Char('r') => StatsWorkflowAction::SwitchMode(StatsMode::ProblemReview),
            KeyCode::Char('g') => StatsWorkflowAction::SwitchMode(StatsMode::TagPractice),
            KeyCode::Esc | KeyCode::Char('b') => {
                StatsWorkflowAction::SwitchMode(StatsMode::Overview)
            }
            KeyCode::Char('q') => StatsWorkflowAction::Exit,
            _ => StatsWorkflowAction::None,
        },
        StatsMode::TagPractice => match key {
            KeyCode::Tab => StatsWorkflowAction::SwitchProvider(next_provider(provider)),
            KeyCode::Char('o') => StatsWorkflowAction::SwitchMode(StatsMode::Overview),
            KeyCode::Char('r') => StatsWorkflowAction::SwitchMode(StatsMode::ProblemReview),
            KeyCode::Char('g') => StatsWorkflowAction::SwitchMode(StatsMode::TagPractice),
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
    provider: &mut BrowserProviderView,
    problem_selected: &mut usize,
    tag_selected: &mut usize,
    action: StatsWorkflowAction,
) -> bool {
    match action {
        StatsWorkflowAction::None => false,
        StatsWorkflowAction::SwitchProvider(next) => {
            *provider = next;
            *problem_selected = 0;
            *tag_selected = 0;
            false
        }
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
    provider: BrowserProviderView,
    mode: StatsMode,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("工作区: {}", workspace_root.display())),
        Line::from(match mode {
            StatsMode::Overview => {
                "[概览模式] 使用 Tab 切换 provider，o/r/g 切换概览 / 题目复习 / 标签加练"
                    .to_string()
            }
            StatsMode::ProblemReview => {
                "[题目复习] 使用 Tab 切换 provider，Esc 返回概览".to_string()
            }
            StatsMode::TagPractice => "[标签加练] 使用 Tab 切换 provider，Esc 返回概览".to_string(),
        }),
        Line::from(format!("Provider: {}", provider_label(provider))),
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
        Line::from(format!("题号: {}", human_problem_id(&item.problem_id))),
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

fn problem_review_browser_query(
    candidate: &ProblemReviewCandidate,
    provider: BrowserProviderView,
) -> BrowserQuery {
    BrowserQuery {
        provider,
        root_view: BrowserRootView::Problems,
        problem_id: Some(candidate.problem_id.clone()),
        timeline_problem: Some(candidate.problem_id.clone()),
        return_to_caller_on_escape: true,
        ..BrowserQuery::default()
    }
}

fn tag_practice_browser_query(
    candidate: &TagPracticeSuggestion,
    provider: BrowserProviderView,
) -> BrowserQuery {
    BrowserQuery {
        provider,
        root_view: BrowserRootView::Problems,
        tag: Some(candidate.tag.clone()),
        return_to_caller_on_escape: true,
        ..BrowserQuery::default()
    }
}

fn default_provider(dashboard: &StatsDashboard) -> BrowserProviderView {
    if dashboard
        .provider_dashboards
        .iter()
        .any(|item| item.provider == BrowserProviderView::Luogu)
    {
        BrowserProviderView::Luogu
    } else if dashboard
        .provider_dashboards
        .iter()
        .any(|item| item.provider == BrowserProviderView::All)
    {
        BrowserProviderView::All
    } else {
        BrowserProviderView::Luogu
    }
}

fn current_stats_dashboard<'a>(
    dashboard: &'a StatsDashboard,
    fallback_dashboards: &'a [StatsProviderDashboard],
    provider: BrowserProviderView,
) -> &'a StatsProviderDashboard {
    dashboard
        .provider_dashboards
        .iter()
        .find(|item| item.provider == provider)
        .or_else(|| {
            fallback_dashboards
                .iter()
                .find(|item| item.provider == provider)
        })
        .or_else(|| dashboard.provider_dashboards.first())
        .or_else(|| fallback_dashboards.first())
        .expect("stats provider dashboards should include at least one provider")
}

fn next_provider(provider: BrowserProviderView) -> BrowserProviderView {
    match provider {
        BrowserProviderView::Luogu => BrowserProviderView::AtCoder,
        BrowserProviderView::AtCoder => BrowserProviderView::All,
        BrowserProviderView::All => BrowserProviderView::Luogu,
    }
}

fn provider_label(provider: BrowserProviderView) -> &'static str {
    match provider {
        BrowserProviderView::Luogu => "Luogu",
        BrowserProviderView::AtCoder => "AtCoder",
        BrowserProviderView::All => "All",
    }
}

fn fallback_provider_dashboards(summary: &StatsSummary) -> Vec<StatsProviderDashboard> {
    [
        (BrowserProviderView::Luogu, true, summary.clone()),
        (
            BrowserProviderView::AtCoder,
            false,
            StatsSummary {
                tag_counts: Vec::new(),
                ..summary.clone()
            },
        ),
        (
            BrowserProviderView::All,
            false,
            StatsSummary {
                tag_counts: Vec::new(),
                ..summary.clone()
            },
        ),
    ]
    .into_iter()
    .map(
        |(provider, tag_features_supported, summary)| StatsProviderDashboard {
            provider,
            summary,
            problem_reviews: Vec::new(),
            tag_practice_suggestions: Vec::new(),
            tag_features_supported,
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::domain::{
        browser::{BrowserProviderView, BrowserRootView},
        stats::{ProblemReviewCandidate, StatsSummary, TagPracticeSuggestion},
    };

    use super::{
        StatsMode, StatsWorkflowAction, apply_stats_workflow_action, distribution_lines,
        fallback_provider_dashboards, problem_review_browser_query,
        selected_problem_review_detail_lines, selected_tag_practice_detail_lines,
        stats_header_lines, stats_overview_lines, stats_workflow_action,
        tag_practice_browser_query,
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
                BrowserProviderView::AtCoder,
                StatsMode::TagPractice
            )
        );
        let overview = format!("{:?}", stats_overview_lines(&summary));
        let empty = format!("{:?}", distribution_lines(&[], "结果分布"));

        assert!(header.contains("[标签加练]"));
        assert!(header.contains("Provider: AtCoder"));
        assert!(header.contains("最近 7 天"));
        assert!(overview.contains("首次 "));
        assert!(overview.contains("\"AC\""));
        assert!(empty.contains("无数据"));
    }

    #[test]
    fn stats_workflow_maps_provider_switch_mode_switch_and_exit_keys() {
        assert_eq!(
            stats_workflow_action(
                StatsMode::Overview,
                BrowserProviderView::Luogu,
                KeyCode::Tab
            ),
            StatsWorkflowAction::SwitchProvider(BrowserProviderView::AtCoder)
        );
        assert_eq!(
            stats_workflow_action(
                StatsMode::ProblemReview,
                BrowserProviderView::AtCoder,
                KeyCode::Tab
            ),
            StatsWorkflowAction::SwitchProvider(BrowserProviderView::All)
        );
        assert_eq!(
            stats_workflow_action(
                StatsMode::TagPractice,
                BrowserProviderView::All,
                KeyCode::Tab
            ),
            StatsWorkflowAction::SwitchProvider(BrowserProviderView::Luogu)
        );
        assert_eq!(
            stats_workflow_action(
                StatsMode::Overview,
                BrowserProviderView::All,
                KeyCode::Char('r')
            ),
            StatsWorkflowAction::SwitchMode(StatsMode::ProblemReview)
        );
        assert_eq!(
            stats_workflow_action(
                StatsMode::ProblemReview,
                BrowserProviderView::Luogu,
                KeyCode::Esc
            ),
            StatsWorkflowAction::SwitchMode(StatsMode::Overview)
        );
        assert_eq!(
            stats_workflow_action(
                StatsMode::TagPractice,
                BrowserProviderView::AtCoder,
                KeyCode::Char('q')
            ),
            StatsWorkflowAction::Exit
        );
    }

    #[test]
    fn switching_stats_mode_or_provider_resets_review_focus() {
        let mut mode = StatsMode::TagPractice;
        let mut provider = BrowserProviderView::Luogu;
        let mut problem_selected = 5usize;
        let mut tag_selected = 7usize;

        assert!(!apply_stats_workflow_action(
            &mut mode,
            &mut provider,
            &mut problem_selected,
            &mut tag_selected,
            StatsWorkflowAction::SwitchMode(StatsMode::Overview),
        ));
        assert_eq!(mode, StatsMode::Overview);
        assert_eq!(provider, BrowserProviderView::Luogu);
        assert_eq!(problem_selected, 0);
        assert_eq!(tag_selected, 0);

        problem_selected = 3;
        tag_selected = 4;
        assert!(!apply_stats_workflow_action(
            &mut mode,
            &mut provider,
            &mut problem_selected,
            &mut tag_selected,
            StatsWorkflowAction::SwitchProvider(BrowserProviderView::AtCoder),
        ));
        assert_eq!(provider, BrowserProviderView::AtCoder);
        assert_eq!(problem_selected, 0);
        assert_eq!(tag_selected, 0);
    }

    #[test]
    fn browser_queries_match_problem_review_and_tag_practice_drill_down() {
        let problem_query = problem_review_browser_query(
            &ProblemReviewCandidate {
                problem_id: "luogu:P1001".to_string(),
                title: "A".to_string(),
                verdict: "WA".to_string(),
                last_submission_time: None,
                priority: 6,
                reasons: vec!["最近状态仍为 WA".to_string()],
                matched_tags: vec!["模拟".to_string()],
            },
            BrowserProviderView::Luogu,
        );
        assert_eq!(problem_query.root_view, BrowserRootView::Problems);
        assert_eq!(problem_query.provider, BrowserProviderView::Luogu);
        assert_eq!(problem_query.problem_id.as_deref(), Some("luogu:P1001"));
        assert_eq!(
            problem_query.timeline_problem.as_deref(),
            Some("luogu:P1001")
        );
        assert!(problem_query.return_to_caller_on_escape);

        let tag_query = tag_practice_browser_query(
            &TagPracticeSuggestion {
                tag: "二分".to_string(),
                recent_unique_problems: 1,
                lifetime_unique_problems: 1,
                priority: 399,
                reason: "最近 60 天仅练过 1 题，建议补样本".to_string(),
                recent_unstable_signal_count: 0,
            },
            BrowserProviderView::AtCoder,
        );
        assert_eq!(tag_query.root_view, BrowserRootView::Problems);
        assert_eq!(tag_query.provider, BrowserProviderView::AtCoder);
        assert_eq!(tag_query.tag.as_deref(), Some("二分"));
        assert_eq!(tag_query.timeline_problem, None);
        assert!(tag_query.return_to_caller_on_escape);
    }

    #[test]
    fn fallback_dashboards_clear_non_luogu_tag_sections() {
        let summary = StatsSummary {
            total_solve_records: 2,
            unique_problem_count: 2,
            unique_ac_count: 1,
            unique_non_ac_count: 1,
            first_ac_count: 1,
            repeated_practice_count: 0,
            time_window_days: None,
            verdict_counts: vec![("AC".to_string(), 1)],
            difficulty_counts: vec![("入门".to_string(), 2)],
            tag_counts: vec![("模拟".to_string(), 2)],
        };

        let dashboards = fallback_provider_dashboards(&summary);
        let atcoder = dashboards
            .iter()
            .find(|item| item.provider == BrowserProviderView::AtCoder)
            .unwrap();
        let all = dashboards
            .iter()
            .find(|item| item.provider == BrowserProviderView::All)
            .unwrap();

        assert!(!atcoder.tag_features_supported);
        assert!(atcoder.summary.tag_counts.is_empty());
        assert!(!all.tag_features_supported);
        assert!(all.summary.tag_counts.is_empty());
    }

    #[test]
    fn selected_detail_helpers_return_empty_states() {
        let problem_empty = format!("{:?}", selected_problem_review_detail_lines(&[], 0));
        let tag_empty = format!("{:?}", selected_tag_practice_detail_lines(&[], 0));

        assert!(problem_empty.contains("当前没有待复习题目"));
        assert!(tag_empty.contains("当前没有建议加练的标签"));
    }
}
