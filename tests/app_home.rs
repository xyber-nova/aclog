mod support;

use std::fs;

use aclog::{app::run_home_with, domain::record::SyncSelection, ui::interaction::HomeAction};

use support::{
    FakeDeps, FakeUi, active_change, sample_metadata, sample_submission, workspace_with_config,
};

#[tokio::test]
async fn home_delivers_local_summary_and_record_rows() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![(
        "rev-1".to_string(),
        "solve(P1001): A\n\nVerdict: AC\nDifficulty: 入门\nSource: Luogu\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: tracked/P1001.cpp".to_string(),
    )]);
    deps.track_file("tracked/P1001.cpp");
    let ui = FakeUi::with_home_actions(vec![HomeAction::Exit]);

    run_home_with(workspace.path().to_path_buf(), &deps, &ui)
        .await
        .unwrap();

    let shown = ui.shown_home_summaries.lock().unwrap();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].total_solve_records, 1);
    assert_eq!(shown[0].unique_problem_count, 1);
    assert_eq!(shown[0].tracked_record_count, 1);
    assert_eq!(shown[0].record_rows.len(), 1);
    assert_eq!(shown[0].record_rows[0].file_name, "tracked/P1001.cpp");
}

#[tokio::test]
async fn home_summary_reports_resumeable_sync_session() {
    let workspace = workspace_with_config();
    fs::write(
        workspace.path().join(".aclog/sync-session.toml"),
        r#"
created_at = "2024-01-03T00:00:00+08:00"

[[items]]
file = "tracked/P1001.cpp"
problem_id = "luogu:P1001"
provider = "luogu"
kind = "Active"
status = "Pending"
submissions = 1
default_submission_id = 1
warnings = []
"#,
    )
    .unwrap();
    let deps = FakeDeps::default();
    let ui = FakeUi::with_home_actions(vec![HomeAction::Exit]);

    run_home_with(workspace.path().to_path_buf(), &deps, &ui)
        .await
        .unwrap();

    let shown = ui.shown_home_summaries.lock().unwrap();
    assert_eq!(shown.len(), 1);
    let sync_session = shown[0]
        .sync_session
        .clone()
        .expect("summary should expose resumable session");
    assert_eq!(sync_session.total_items, 1);
    assert_eq!(sync_session.pending_items, 1);
}

#[tokio::test]
async fn home_can_dispatch_to_sync_stats_and_browser_workflows() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P1001.cpp")]);
    deps.insert_metadata("luogu:P1001", Some(sample_metadata("luogu:P1001")));
    deps.insert_submissions("luogu:P1001", vec![sample_submission(42, "AC")]);
    deps.set_commit_descriptions(vec![(
        "rev-1".to_string(),
        "solve(P1002): B\n\nVerdict: WA\nDifficulty: 入门\nTags: 模拟\nSubmission-ID: 2\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: tracked/P1002.cpp".to_string(),
    )]);
    deps.set_algorithm_tag_names(&["模拟"]);

    let ui = FakeUi::with_home_actions(vec![
        HomeAction::StartSync,
        HomeAction::OpenStats,
        HomeAction::OpenBrowserFiles,
        HomeAction::OpenBrowserProblems,
        HomeAction::Exit,
    ]);
    *ui.submission_selection.lock().unwrap() = Some(SyncSelection::Skip);

    run_home_with(workspace.path().to_path_buf(), &deps, &ui)
        .await
        .unwrap();

    assert_eq!(deps.submission_fetch_count("luogu:P1001"), 1);
    assert_eq!(ui.shown_dashboards.lock().unwrap().len(), 1);

    let opened = ui.opened_browsers.lock().unwrap();
    assert_eq!(opened.len(), 2);
    assert_eq!(opened[0].root_view, aclog::app::BrowserRootView::Files);
    assert_eq!(opened[1].root_view, aclog::app::BrowserRootView::Problems);
}
