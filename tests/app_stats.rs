mod support;

use aclog::app::{StatsOptions, run_stats_with, run_stats_with_options_and_deps};

use support::{FakeDeps, FakeUi, workspace_with_config};

#[tokio::test]
async fn stats_delivers_filtered_summary_to_ui() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![
        (
            "rev-1".to_string(),
            "solve(P1001): A\n\nVerdict: AC\nDifficulty: 入门\nTags: 模拟, 年份\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: P1001.cpp".to_string(),
        ),
        (
            "rev-2".to_string(),
            "solve(P1001): A\n\nVerdict: WA\nDifficulty: 入门\nTags: 模拟\nSubmission-ID: 2\nSubmission-Time: 2024-01-01T03:04:05+08:00\nFile: P1001.cpp".to_string(),
        ),
        (
            "rev-3".to_string(),
            "solve(P1002): B\n\nVerdict: WA\nDifficulty: 普及-\nTags: 二分\nSubmission-ID: 3\nSubmission-Time: 2024-01-03T03:04:05+08:00\nFile: P1002.cpp".to_string(),
        ),
    ]);
    deps.set_algorithm_tag_names(&["模拟", "二分"]);
    let ui = FakeUi::default();

    run_stats_with(workspace.path().to_path_buf(), &deps, &ui)
        .await
        .unwrap();

    let shown = ui.shown_dashboards.lock().unwrap();
    assert_eq!(shown.len(), 1);
    let summary = &shown[0].summary;
    assert_eq!(summary.total_solve_records, 3);
    assert_eq!(summary.unique_problem_count, 2);
    assert_eq!(summary.unique_ac_count, 1);
    assert_eq!(
        summary.tag_counts,
        vec![("二分".to_string(), 1), ("模拟".to_string(), 1)]
    );
    assert_eq!(summary.first_ac_count, 1);
    assert_eq!(summary.repeated_practice_count, 1);
}

#[tokio::test]
async fn stats_review_mode_opens_dashboard_in_review_mode() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![
        (
            "rev-a".to_string(),
            "solve(P1001): A\n\nVerdict: WA\nDifficulty: 入门\nTags: 模拟\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: P1001.cpp\nMistakes: 边界".to_string(),
        ),
        (
            "rev-b".to_string(),
            "solve(P1002): B\n\nVerdict: AC\nDifficulty: 普及-\nTags: 二分\nSubmission-ID: 3\nSubmission-Time: 2024-01-03T03:04:05+08:00\nFile: P1002.cpp".to_string(),
        ),
    ]);
    deps.set_algorithm_tag_names(&["模拟", "二分"]);
    let ui = FakeUi::default();

    run_stats_with_options_and_deps(
        workspace.path().to_path_buf(),
        StatsOptions {
            review: true,
            ..StatsOptions::default()
        },
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let dashboards = ui.shown_dashboards.lock().unwrap();
    assert_eq!(dashboards.len(), 1);
    assert!(dashboards[0].start_in_review);
    assert!(
        dashboards[0]
            .review_candidates
            .iter()
            .any(|item| item.kind == "retry" && item.label == "P1001")
    );
}
