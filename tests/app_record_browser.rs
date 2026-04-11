mod support;

use aclog::app::{BrowserProviderView, BrowserQuery, BrowserRootView, run_record_browse_with};

use support::{FakeDeps, FakeUi, workspace_with_config};

#[tokio::test]
async fn record_browse_json_filters_file_view() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![
        (
            "rev-a".to_string(),
            "solve(P1001): title\n\nVerdict: AC\nDifficulty: 入门\nTags: 模拟\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: tracked/P1001.cpp".to_string(),
        ),
        (
            "rev-b".to_string(),
            "solve(P1002): title\n\nVerdict: WA\nDifficulty: 普及-\nTags: 二分\nSubmission-ID: 2\nSubmission-Time: 2024-01-03T03:04:05+08:00\nFile: tracked/P1002.cpp".to_string(),
        ),
    ]);
    let ui = FakeUi::default();

    run_record_browse_with(
        workspace.path().to_path_buf(),
        BrowserQuery {
            root_view: BrowserRootView::Files,
            verdict: Some("AC".to_string()),
            json: true,
            ..BrowserQuery::default()
        },
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let output = deps.outputs().join("");
    assert!(output.contains("\"problem_id\": \"luogu:P1001\""));
    assert!(!output.contains("\"problem_id\": \"luogu:P1002\""));
}

#[tokio::test]
async fn record_browse_non_json_opens_browser_workbench() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![(
        "rev-a".to_string(),
        "solve(P1001): title\n\nVerdict: AC\nDifficulty: 入门\nTags: 模拟\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: tracked/P1001.cpp".to_string(),
    )]);
    let ui = FakeUi::default();

    run_record_browse_with(
        workspace.path().to_path_buf(),
        BrowserQuery {
            root_view: BrowserRootView::Problems,
            problem_id: Some("P1001".to_string()),
            ..BrowserQuery::default()
        },
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let opened = ui.opened_browsers.lock().unwrap();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].root_view, BrowserRootView::Problems);
    assert_eq!(opened[0].problem_id.as_deref(), Some("P1001"));
}

#[tokio::test]
async fn record_browse_json_filters_to_atcoder_provider() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![
        (
            "rev-l".to_string(),
            "solve(P1001): title\n\nVerdict: AC\nDifficulty: 入门\nTags: 模拟\nSource: Luogu\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: tracked/P1001.cpp".to_string(),
        ),
        (
            "rev-a".to_string(),
            "solve(atcoder:abc350_a): title\n\nVerdict: WA\nDifficulty: C\nTags: implementation\nSource: AtCoder\nContest: ABC350\nSubmission-ID: 2\nSubmission-Time: 2024-01-03T03:04:05+08:00\nFile: tracked/abc350_a.cpp".to_string(),
        ),
    ]);
    let ui = FakeUi::default();

    run_record_browse_with(
        workspace.path().to_path_buf(),
        BrowserQuery {
            provider: BrowserProviderView::AtCoder,
            root_view: BrowserRootView::Problems,
            json: true,
            ..BrowserQuery::default()
        },
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let output = deps.outputs().join("");
    assert!(output.contains("\"problem_id\": \"atcoder:abc350_a\""));
    assert!(output.contains("\"contest\": \"ABC350\""));
    assert!(!output.contains("\"problem_id\": \"luogu:P1001\""));
}
