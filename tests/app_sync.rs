mod support;

use aclog::{
    app::run_sync_with, commit_format::build_solve_commit_message, domain::record::SyncSelection,
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

    run_sync_with(workspace.path().to_path_buf(), &deps, &ui)
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

    run_sync_with(workspace.path().to_path_buf(), &deps, &ui)
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

    run_sync_with(workspace.path().to_path_buf(), &deps, &ui)
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

    run_sync_with(workspace.path().to_path_buf(), &deps, &ui)
        .await
        .unwrap();

    assert!(deps.created_commits().is_empty());
}
