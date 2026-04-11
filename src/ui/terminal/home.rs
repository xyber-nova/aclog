//! 全局训练工作台首页。

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Constraint,
    text::{Line, Span},
    widgets::{Cell, List, ListItem, ListState, Row, Table, TableState},
};

use crate::{
    problem::{human_problem_id, provider_label},
    ui::{
        interaction::{HomeAction, HomeRecordListRow, HomeSummary},
        terminal::{
            TerminalHandle,
            common::{
                clamp_selection, footer_panel, initial_selection_for_count, is_help, lines_panel,
                move_selection, root_vertical_layout, split_main_with_summary,
            },
            theme,
        },
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomeScreen {
    Main,
    RecordList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomeEntryKey {
    ResumeSync,
    StartSync,
    Stats,
    BrowseFiles,
    BrowseProblems,
    RecordList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomeWorkflowAction {
    None,
    OpenRecordList,
    BackToMain,
    Exit,
    Launch(HomeAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HomeEntry {
    key: HomeEntryKey,
    title: &'static str,
}

pub(crate) fn run_home_app(
    terminal: &mut TerminalHandle,
    workspace_root: &std::path::Path,
    summary: &HomeSummary,
) -> Result<HomeAction> {
    let entries = home_entries(summary);
    let mut screen = HomeScreen::Main;
    let mut main_selected = initial_selection_for_count(entries.len()).unwrap_or_default();
    let mut record_selected = initial_selection_for_count(summary.record_rows.len()).unwrap_or(0);
    let mut help_visible = false;

    loop {
        terminal.draw(|frame| {
            let [header, content, footer] = root_vertical_layout(frame.area(), 5);

            frame.render_widget(
                lines_panel("训练工作台首页", home_header_lines(workspace_root, summary)),
                header,
            );

            match screen {
                HomeScreen::Main => {
                    let [list_area, detail_area] = split_main_with_summary(content, 38, 62);
                    render_home_entries(frame, list_area, &entries, main_selected);
                    let detail = entries
                        .get(clamp_selection(main_selected, entries.len()))
                        .map(|entry| entry_detail_lines(*entry, summary))
                        .unwrap_or_else(|| vec![Line::from("当前没有可用入口")]);
                    frame.render_widget(lines_panel("入口说明", detail), detail_area);
                }
                HomeScreen::RecordList => {
                    let [table_area, detail_area] = split_main_with_summary(content, 66, 34);
                    render_record_list_table(
                        frame,
                        table_area,
                        &summary.record_rows,
                        record_selected,
                    );
                    let detail = summary
                        .record_rows
                        .get(clamp_selection(record_selected, summary.record_rows.len()))
                        .map(record_detail_lines)
                        .unwrap_or_else(|| vec![Line::from("当前工作区还没有已记录的解法文件")]);
                    frame.render_widget(lines_panel("记录详情", detail), detail_area);
                }
            }

            let (core, help_lines): (&str, &[&str]) = match screen {
                HomeScreen::Main => (
                    "j/k/↑/↓ 移动  Enter 打开入口  Esc 退出  q 退出",
                    &[
                        "Enter: 打开当前入口",
                        "Esc: 退出首页",
                        "q: 直接退出 aclog",
                        "?: 切换帮助",
                        "提示: 现有子命令仍可直接使用，例如 aclog sync / aclog stats",
                    ],
                ),
                HomeScreen::RecordList => (
                    "Esc 返回首页  q 退出  ? 帮助",
                    &["Esc: 返回首页", "q: 退出 aclog", "?: 切换帮助"],
                ),
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
        match home_workflow_action(
            screen,
            entries
                .get(clamp_selection(main_selected, entries.len()))
                .map(|entry| entry.key),
            key.code,
        ) {
            HomeWorkflowAction::None => {}
            HomeWorkflowAction::OpenRecordList => {
                screen = HomeScreen::RecordList;
                record_selected =
                    initial_selection_for_count(summary.record_rows.len()).unwrap_or(0);
            }
            HomeWorkflowAction::BackToMain => {
                screen = HomeScreen::Main;
            }
            HomeWorkflowAction::Exit => return Ok(HomeAction::Exit),
            HomeWorkflowAction::Launch(action) => return Ok(action),
        }

        match screen {
            HomeScreen::Main => {
                if let Some(Some(next)) =
                    move_selection(key.code, Some(main_selected), entries.len())
                {
                    main_selected = next;
                }
            }
            HomeScreen::RecordList => {
                if let Some(Some(next)) =
                    move_selection(key.code, Some(record_selected), summary.record_rows.len())
                {
                    record_selected = next;
                }
            }
        }
    }
}

fn home_entries(summary: &HomeSummary) -> Vec<HomeEntry> {
    let mut entries = Vec::new();
    if summary.sync_session.is_some() {
        entries.push(HomeEntry {
            key: HomeEntryKey::ResumeSync,
            title: "恢复 sync 批次",
        });
    }
    entries.extend([
        HomeEntry {
            key: HomeEntryKey::StartSync,
            title: "开始 sync",
        },
        HomeEntry {
            key: HomeEntryKey::Stats,
            title: "训练统计",
        },
        HomeEntry {
            key: HomeEntryKey::BrowseFiles,
            title: "文件浏览",
        },
        HomeEntry {
            key: HomeEntryKey::BrowseProblems,
            title: "题目浏览",
        },
        HomeEntry {
            key: HomeEntryKey::RecordList,
            title: "记录列表",
        },
    ]);
    entries
}

fn home_header_lines(
    workspace_root: &std::path::Path,
    summary: &HomeSummary,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("工作区: {}", workspace_root.display())),
        Line::from(format!(
            "本地历史: {} 条 solve / {} 道题 / {} 个 tracked 文件记录",
            summary.total_solve_records, summary.unique_problem_count, summary.tracked_record_count
        )),
    ];

    if summary.provider_summaries.is_empty() {
        lines.push(Line::from("来源摘要: 当前还没有可识别的本地训练记录"));
    } else {
        lines.push(Line::from(format!(
            "来源摘要: {}",
            summary
                .provider_summaries
                .iter()
                .map(|item| format!(
                    "{} {}题/{}条",
                    provider_label(item.provider),
                    item.unique_problem_count,
                    item.total_solve_records
                ))
                .collect::<Vec<_>>()
                .join(" · ")
        )));
    }

    if let Some(session) = &summary.sync_session {
        lines.push(Line::from(format!(
            "恢复状态: 可恢复批次（{} 项，待处理 {}，已决 {}，创建于 {}）",
            session.total_items,
            session.pending_items,
            session.decided_items,
            session.created_at.format("%Y-%m-%d %H:%M")
        )));
    } else {
        lines.push(Line::from("恢复状态: 当前没有未完成的 sync 批次"));
    }
    lines
}

fn entry_detail_lines(entry: HomeEntry, summary: &HomeSummary) -> Vec<Line<'static>> {
    match entry.key {
        HomeEntryKey::ResumeSync => {
            let Some(session) = summary.sync_session.as_ref() else {
                return vec![Line::from("当前没有可恢复的 sync 批次")];
            };
            vec![
                Line::from("继续上一次未完成的工作区记账流程。"),
                Line::from(format!(
                    "当前批次共 {} 项，其中待处理 {} 项。",
                    session.total_items, session.pending_items
                )),
                Line::from(format!(
                    "创建时间: {}",
                    session.created_at.format("%Y-%m-%d %H:%M")
                )),
            ]
        }
        HomeEntryKey::StartSync => vec![
            Line::from("按当前工作副本重新检测题目文件变更。"),
            Line::from("适合开始新一轮记账，或在不恢复旧批次时重建 sync。"),
        ],
        HomeEntryKey::Stats => {
            let mut lines = vec![
                Line::from("打开训练统计工作台，查看 overview / review / tag practice。"),
                Line::from(format!(
                    "当前本地历史共 {} 条 solve，覆盖 {} 道题。",
                    summary.total_solve_records, summary.unique_problem_count
                )),
            ];
            if let Some(record) = &summary.latest_record {
                lines.push(Line::from(format!(
                    "最近记录: {} · {} · {}",
                    human_problem_id(&record.problem_id),
                    record.file_name,
                    record.verdict
                )));
            }
            lines
        }
        HomeEntryKey::BrowseFiles => vec![
            Line::from("以文件视角浏览当前记录状态，再钻取到单文件时间线。"),
            Line::from("适合从“这份题解现在是什么状态”开始回看历史。"),
        ],
        HomeEntryKey::BrowseProblems => vec![
            Line::from("以题目视角浏览当前状态，再钻取到单题时间线。"),
            Line::from("适合同一题存在多份文件记录时回看整体脉络。"),
        ],
        HomeEntryKey::RecordList => vec![
            Line::from("查看当前仍被工作区跟踪的记录列表快照。"),
            Line::from(format!(
                "当前快照包含 {} 个 tracked 文件记录，使用与 `aclog record list` 相同的解释口径。",
                summary.tracked_record_count
            )),
        ],
    }
}

fn render_home_entries(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    entries: &[HomeEntry],
    selected: usize,
) {
    let items = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let prefix = if index == selected {
                theme::FOCUS_SYMBOL
            } else {
                "  "
            };
            ListItem::new(Line::from(vec![Span::raw(prefix), Span::raw(entry.title)]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(initial_selection_for_count(entries.len()).map(|_| selected));
    let list = List::new(items)
        .block(super::common::panel("入口"))
        .highlight_style(theme::selected_row_style())
        .highlight_symbol("");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_record_list_table(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    rows: &[HomeRecordListRow],
    selected: usize,
) {
    if rows.is_empty() {
        frame.render_widget(
            lines_panel(
                "记录列表",
                vec![Line::from("当前工作区还没有已记录的解法文件")],
            ),
            area,
        );
        return;
    }

    let table_rows = rows
        .iter()
        .map(|row| {
            let submission_id = row
                .submission_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let recorded_at = row
                .submission_time
                .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            Row::new(vec![
                Cell::from(row.file_name.clone()),
                Cell::from(human_problem_id(&row.problem_id)),
                Cell::from(row.verdict.clone()).style(theme::verdict_style(&row.verdict)),
                Cell::from(row.difficulty.clone()),
                Cell::from(submission_id),
                Cell::from(recorded_at),
            ])
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        table_rows,
        [
            Constraint::Length(18),
            Constraint::Length(16),
            Constraint::Length(6),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(18),
        ],
    )
    .header(
        Row::new(vec!["文件", "题号", "结果", "难度", "提交编号", "记录时间"])
            .style(theme::accent_style()),
    )
    .block(super::common::panel("记录列表"))
    .row_highlight_style(theme::selected_row_style())
    .highlight_symbol(theme::FOCUS_SYMBOL);

    let mut state = TableState::default();
    state.select(initial_selection_for_count(rows.len()).map(|_| selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn record_detail_lines(row: &HomeRecordListRow) -> Vec<Line<'static>> {
    vec![
        Line::from(format!("文件: {}", row.file_name)),
        Line::from(format!("题号: {}", human_problem_id(&row.problem_id))),
        Line::from(format!("结果: {}", row.verdict)),
        Line::from(format!("难度: {}", row.difficulty)),
        Line::from(format!(
            "提交编号: {}",
            row.submission_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        )),
        Line::from(format!(
            "记录时间: {}",
            row.submission_time
                .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string())
        )),
        Line::from(""),
        Line::from("标题"),
        Line::from(row.title.clone()),
    ]
}

fn home_workflow_action(
    screen: HomeScreen,
    selected_entry: Option<HomeEntryKey>,
    key: KeyCode,
) -> HomeWorkflowAction {
    match screen {
        HomeScreen::Main => match key {
            KeyCode::Enter => match selected_entry {
                Some(HomeEntryKey::ResumeSync) => {
                    HomeWorkflowAction::Launch(HomeAction::ResumeSync)
                }
                Some(HomeEntryKey::StartSync) => HomeWorkflowAction::Launch(HomeAction::StartSync),
                Some(HomeEntryKey::Stats) => HomeWorkflowAction::Launch(HomeAction::OpenStats),
                Some(HomeEntryKey::BrowseFiles) => {
                    HomeWorkflowAction::Launch(HomeAction::OpenBrowserFiles)
                }
                Some(HomeEntryKey::BrowseProblems) => {
                    HomeWorkflowAction::Launch(HomeAction::OpenBrowserProblems)
                }
                Some(HomeEntryKey::RecordList) => HomeWorkflowAction::OpenRecordList,
                None => HomeWorkflowAction::None,
            },
            KeyCode::Esc | KeyCode::Char('q') => HomeWorkflowAction::Exit,
            _ => HomeWorkflowAction::None,
        },
        HomeScreen::RecordList => match key {
            KeyCode::Esc => HomeWorkflowAction::BackToMain,
            KeyCode::Char('q') => HomeWorkflowAction::Exit,
            _ => HomeWorkflowAction::None,
        },
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use super::{
        HomeEntryKey, HomeScreen, HomeWorkflowAction, entry_detail_lines, home_entries,
        home_header_lines, home_workflow_action,
    };
    use crate::{
        problem::ProblemProvider,
        ui::interaction::{
            HomeAction, HomeLatestRecordSummary, HomeProviderSummary, HomeRecordListRow,
            HomeSummary, HomeSyncSessionSummary,
        },
    };
    use chrono::{FixedOffset, TimeZone};

    fn sample_summary(with_session: bool) -> HomeSummary {
        HomeSummary {
            total_solve_records: 3,
            unique_problem_count: 2,
            tracked_record_count: 1,
            provider_summaries: vec![HomeProviderSummary {
                provider: ProblemProvider::Luogu,
                total_solve_records: 3,
                unique_problem_count: 2,
            }],
            latest_record: Some(HomeLatestRecordSummary {
                problem_id: "luogu:P1001".to_string(),
                provider: ProblemProvider::Luogu,
                title: "A".to_string(),
                file_name: "tracked/P1001.cpp".to_string(),
                verdict: "AC".to_string(),
                submission_time: None,
            }),
            sync_session: with_session.then(|| HomeSyncSessionSummary {
                created_at: FixedOffset::east_opt(8 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2024, 1, 3, 0, 0, 0)
                    .single()
                    .unwrap(),
                total_items: 3,
                pending_items: 2,
                decided_items: 1,
            }),
            record_rows: vec![HomeRecordListRow {
                file_name: "tracked/P1001.cpp".to_string(),
                problem_id: "luogu:P1001".to_string(),
                verdict: "AC".to_string(),
                difficulty: "入门".to_string(),
                submission_id: Some(1),
                submission_time: None,
                title: "A".to_string(),
            }],
        }
    }

    #[test]
    fn home_entries_include_resume_only_when_session_exists() {
        let with_session = home_entries(&sample_summary(true));
        let without_session = home_entries(&sample_summary(false));

        assert_eq!(with_session[0].key, HomeEntryKey::ResumeSync);
        assert!(
            without_session
                .iter()
                .all(|entry| entry.key != HomeEntryKey::ResumeSync)
        );
    }

    #[test]
    fn home_header_lines_and_detail_lines_render_sync_and_snapshot_context() {
        let summary = sample_summary(true);
        let header = format!(
            "{:?}",
            home_header_lines(std::path::Path::new("/tmp/ws"), &summary)
        );
        let detail = format!(
            "{:?}",
            entry_detail_lines(home_entries(&summary)[0], &summary)
        );

        assert!(header.contains("可恢复批次"));
        assert!(header.contains("/tmp/ws"));
        assert!(detail.contains("待处理 2 项"));
    }

    #[test]
    fn home_workflow_action_maps_enter_and_escape_by_screen() {
        assert_eq!(
            home_workflow_action(HomeScreen::Main, Some(HomeEntryKey::Stats), KeyCode::Enter),
            HomeWorkflowAction::Launch(HomeAction::OpenStats)
        );
        assert_eq!(
            home_workflow_action(
                HomeScreen::Main,
                Some(HomeEntryKey::RecordList),
                KeyCode::Enter
            ),
            HomeWorkflowAction::OpenRecordList
        );
        assert_eq!(
            home_workflow_action(HomeScreen::RecordList, None, KeyCode::Esc),
            HomeWorkflowAction::BackToMain
        );
        assert_eq!(
            home_workflow_action(HomeScreen::Main, None, KeyCode::Char('q')),
            HomeWorkflowAction::Exit
        );
    }
}
