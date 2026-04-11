mod support;

use std::fs;

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
            .problem_reviews
            .iter()
            .any(|item| item.problem_id == "P1001" && item.verdict == "WA")
    );
    assert!(
        dashboards[0]
            .tag_practice_suggestions
            .iter()
            .any(|item| item.tag == "模拟")
    );
}

#[tokio::test]
async fn stats_review_json_uses_grouped_output_and_legacy_config_defaults() {
    let workspace = tempfile::tempdir().unwrap();
    let aclog_dir = workspace.path().join(".aclog");
    fs::create_dir_all(aclog_dir.join("problems")).unwrap();
    fs::write(
        aclog_dir.join("config.toml"),
        "[user]\nluogu_uid = \"123\"\nluogu_cookie = \"cookie\"\n\n[settings]\nmetadata_ttl_days = 7\nproblem_metadata_ttl_days = 7\nluogu_mappings_ttl_days = 7\nluogu_tags_ttl_days = 7\n",
    )
    .unwrap();

    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![(
        "rev-old".to_string(),
        "solve(P2001): Stable\n\nVerdict: AC\nDifficulty: 入门\nTags: 模拟\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: P2001.cpp".to_string(),
    )]);
    deps.set_algorithm_tag_names(&["模拟"]);
    let ui = FakeUi::default();

    run_stats_with_options_and_deps(
        workspace.path().to_path_buf(),
        StatsOptions {
            review: true,
            json: true,
            ..StatsOptions::default()
        },
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let outputs = deps.outputs();
    assert_eq!(outputs.len(), 1);

    let value: serde_json::Value = serde_json::from_str(outputs[0].trim()).unwrap();
    assert!(value.get("problem_reviews").is_some());
    assert!(value.get("tag_practice_suggestions").is_some());
    assert!(value.get("review_candidates").is_none());
    assert_eq!(value["problem_reviews"].as_array().unwrap().len(), 1);
    assert_eq!(
        value["tag_practice_suggestions"].as_array().unwrap().len(),
        1
    );
}

#[tokio::test]
async fn stats_days_filters_summary_but_not_review_suggestions() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![
        (
            "rev-old".to_string(),
            "solve(P3001): Old\n\nVerdict: AC\nDifficulty: 入门\nTags: 模拟\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: P3001.cpp".to_string(),
        ),
        (
            "rev-new".to_string(),
            "solve(P3002): New\n\nVerdict: AC\nDifficulty: 普及-\nTags: 二分\nSubmission-ID: 2\nSubmission-Time: 2099-01-02T03:04:05+08:00\nFile: P3002.cpp".to_string(),
        ),
    ]);
    deps.set_algorithm_tag_names(&["模拟", "二分"]);
    let ui = FakeUi::default();

    run_stats_with_options_and_deps(
        workspace.path().to_path_buf(),
        StatsOptions {
            days: Some(7),
            ..StatsOptions::default()
        },
        &deps,
        &ui,
    )
    .await
    .unwrap();

    run_stats_with_options_and_deps(
        workspace.path().to_path_buf(),
        StatsOptions {
            days: Some(7),
            review: true,
            ..StatsOptions::default()
        },
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let dashboards = ui.shown_dashboards.lock().unwrap();
    assert_eq!(dashboards.len(), 2);
    assert_eq!(dashboards[0].summary.total_solve_records, 1);
    assert_eq!(dashboards[1].summary.total_solve_records, 1);
    assert!(
        dashboards[1]
            .problem_reviews
            .iter()
            .any(|item| item.problem_id == "P3001")
    );
    assert!(
        dashboards[1]
            .tag_practice_suggestions
            .iter()
            .any(|item| item.tag == "模拟")
    );
}
