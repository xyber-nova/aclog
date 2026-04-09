mod support;

use aclog::{
    app::{SyncOptions, run_sync_with_full_options, run_sync_with_options},
    commit_format::build_solve_commit_message,
    domain::record::SyncSelection,
};

use support::{
    FakeDeps, FakeUi, active_change, deleted_change, sample_metadata, sample_submission,
    workspace_with_config,
};

#[tokio::test]
async fn sync_skips_unparseable_files_and_commits_selected_submission() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("notes.txt"), active_change("P1001.cpp")]);
    deps.insert_metadata("P1001", Some(sample_metadata("P1001")));
    let submission = sample_submission(42, "AC");
    deps.insert_submissions("P1001", vec![submission.clone()]);
    let ui = FakeUi::with_submission_selection(SyncSelection::Submission(submission.clone()));

    run_sync_with_options(workspace.path().to_path_buf(), false, &deps, &ui)
        .await
        .unwrap();

    let commits = deps.created_commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].0, "P1001.cpp");
    assert_eq!(
        commits[0].1,
        build_solve_commit_message(
            "P1001",
            "P1001.cpp",
            Some(&sample_metadata("P1001")),
            &submission
        )
    );
}

#[tokio::test]
async fn sync_creates_chore_commit_when_user_selects_chore() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P1002.cpp")]);
    deps.insert_metadata("P1002", Some(sample_metadata("P1002")));
    deps.insert_submissions("P1002", vec![]);
    let ui = FakeUi::with_submission_selection(SyncSelection::Chore);

    run_sync_with_options(workspace.path().to_path_buf(), false, &deps, &ui)
        .await
        .unwrap();

    let commits = deps.created_commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].0, "P1002.cpp");
    assert!(commits[0].1.starts_with("chore(P1002):"));
}

#[tokio::test]
async fn sync_creates_delete_commit_for_deleted_files() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![deleted_change("P1003.cpp")]);
    deps.insert_metadata("P1003", Some(sample_metadata("P1003")));
    let ui = FakeUi::with_delete_confirmation(SyncSelection::Delete);

    run_sync_with_options(workspace.path().to_path_buf(), false, &deps, &ui)
        .await
        .unwrap();

    let commits = deps.created_commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].0, "P1003.cpp");
    assert!(commits[0].1.starts_with("remove(P1003):"));
}

#[tokio::test]
async fn sync_skip_produces_no_commit() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P1004.cpp")]);
    deps.insert_metadata("P1004", Some(sample_metadata("P1004")));
    deps.insert_submissions("P1004", vec![sample_submission(1, "WA")]);
    let ui = FakeUi::with_submission_selection(SyncSelection::Skip);

    run_sync_with_options(workspace.path().to_path_buf(), false, &deps, &ui)
        .await
        .unwrap();

    assert!(deps.created_commits().is_empty());
}

#[tokio::test]
async fn sync_dry_run_outputs_preview_without_commits_or_ui() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P1005.cpp"), active_change("notes.txt")]);
    deps.insert_metadata("P1005", Some(sample_metadata("P1005")));
    deps.insert_submissions(
        "P1005",
        vec![sample_submission(10, "AC"), sample_submission(9, "WA")],
    );
    let ui = FakeUi::default();

    run_sync_with_options(workspace.path().to_path_buf(), true, &deps, &ui)
        .await
        .unwrap();

    assert!(deps.created_commits().is_empty());
    let output = deps.outputs().join("");
    assert!(output.contains("P1005.cpp"));
    assert!(output.contains("等待选择提交记录"));
    assert!(output.contains("notes.txt"));
    assert!(output.contains("无法识别题号"));
}

#[tokio::test]
async fn sync_resume_uses_saved_session_and_clears_it_after_commit() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P2001.cpp")]);
    deps.insert_metadata("P2001", Some(sample_metadata("P2001")));
    let submission = sample_submission(66, "AC");
    deps.insert_submissions("P2001", vec![submission.clone()]);

    let paused_ui = FakeUi {
        sync_batch_review_selection: std::sync::Mutex::new(vec![None]),
        submission_selection: std::sync::Mutex::new(Some(SyncSelection::Submission(
            submission.clone(),
        ))),
        ..FakeUi::default()
    };
    run_sync_with_full_options(
        workspace.path().to_path_buf(),
        SyncOptions::default(),
        &deps,
        &paused_ui,
    )
    .await
    .unwrap();
    assert!(workspace.path().join(".aclog/sync-session.toml").exists());
    assert!(deps.created_commits().is_empty());

    let resumed_ui = FakeUi {
        submission_selection: std::sync::Mutex::new(Some(SyncSelection::Submission(
            submission.clone(),
        ))),
        ..FakeUi::default()
    };
    run_sync_with_full_options(
        workspace.path().to_path_buf(),
        SyncOptions {
            resume: true,
            ..SyncOptions::default()
        },
        &deps,
        &resumed_ui,
    )
    .await
    .unwrap();

    assert_eq!(deps.created_commits().len(), 1);
    assert!(!workspace.path().join(".aclog/sync-session.toml").exists());
}

#[tokio::test]
async fn sync_duplicate_submission_requires_explicit_second_confirmation() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P2002.cpp")]);
    deps.insert_metadata("P2002", Some(sample_metadata("P2002")));
    let submission = sample_submission(77, "AC");
    deps.insert_submissions("P2002", vec![submission.clone()]);
    deps.set_commit_descriptions(vec![(
        "rev-old".to_string(),
        "solve(P2002): title\n\nVerdict: AC\nSubmission-ID: 77\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: P2002.cpp".to_string(),
    )]);
    let ui = FakeUi {
        submission_selection: std::sync::Mutex::new(Some(SyncSelection::Submission(
            submission.clone(),
        ))),
        ..FakeUi::default()
    };

    run_sync_with_full_options(
        workspace.path().to_path_buf(),
        SyncOptions::default(),
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let commits = deps.created_commits();
    assert_eq!(commits.len(), 1);
    assert!(commits[0].1.contains("Submission-ID: 77"));
}
