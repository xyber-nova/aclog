mod support;

use aclog::app::run_record_rebind_with;

use support::{
    FakeDeps, FakeUi, sample_history_record, sample_metadata, sample_submission,
    workspace_with_config, write_workspace_file,
};

#[tokio::test]
async fn record_rebind_uses_cli_revision_and_submission_id() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1001.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1001.cpp");
    deps.set_commit_descriptions(vec![(
        "real-rev".to_string(),
        "solve(P1001): title\n\nSubmission-ID: 1\nFile: P1001.cpp".to_string(),
    )]);
    deps.resolve_revset_as("abc123", "real-rev");
    deps.insert_metadata("P1001", Some(sample_metadata("P1001")));
    deps.insert_submissions("P1001", vec![sample_submission(2, "AC")]);
    let ui = FakeUi::default();

    run_record_rebind_with(
        workspace.path().to_path_buf(),
        workspace.path().join("P1001.cpp"),
        Some("abc123".to_string()),
        Some(2),
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let rewritten = deps.rewritten_descriptions();
    assert_eq!(rewritten.len(), 1);
    assert_eq!(rewritten[0].0, "real-rev");
    assert!(rewritten[0].1.contains("Submission-ID: 2"));
}

#[tokio::test]
async fn record_rebind_falls_back_to_ui_for_remaining_choices() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1002.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1002.cpp");
    deps.set_commit_descriptions(vec![(
        "rev-ui".to_string(),
        "solve(P1002): title\n\nSubmission-ID: 1\nFile: P1002.cpp".to_string(),
    )]);
    deps.insert_metadata("P1002", Some(sample_metadata("P1002")));
    let submission = sample_submission(8, "AC");
    deps.insert_submissions("P1002", vec![submission.clone()]);
    let ui = FakeUi {
        record_to_rebind_selection: std::sync::Mutex::new(Some(Some(sample_history_record(
            "rev-ui",
            "P1002",
            "P1002.cpp",
            Some(1),
            "WA",
        )))),
        record_submission_selection: std::sync::Mutex::new(Some(Some(submission))),
        ..FakeUi::default()
    };

    run_record_rebind_with(
        workspace.path().to_path_buf(),
        workspace.path().join("P1002.cpp"),
        None,
        None,
        &deps,
        &ui,
    )
    .await
    .unwrap();

    assert_eq!(deps.rewritten_descriptions().len(), 1);
}

#[tokio::test]
async fn record_rebind_rejects_revision_outside_candidate_set() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1003.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1003.cpp");
    deps.set_commit_descriptions(vec![(
        "real-rev".to_string(),
        "solve(P1004): title\n\nSubmission-ID: 1\nFile: P1003.cpp".to_string(),
    )]);
    deps.resolve_revset_as("abc123", "real-rev");
    deps.insert_metadata("P1003", Some(sample_metadata("P1003")));
    deps.insert_submissions("P1003", vec![sample_submission(2, "AC")]);
    let ui = FakeUi::default();

    let error = run_record_rebind_with(
        workspace.path().to_path_buf(),
        workspace.path().join("P1003.cpp"),
        Some("abc123".to_string()),
        Some(2),
        &deps,
        &ui,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("当前没有可重绑的记录"));
}
