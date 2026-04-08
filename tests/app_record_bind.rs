mod support;

use aclog::{app::run_record_bind_with, commit_format::build_solve_commit_message};

use support::{
    FakeDeps, FakeUi, sample_metadata, sample_submission, workspace_with_config,
    write_workspace_file,
};

#[tokio::test]
async fn record_bind_uses_cli_submission_id_without_ui() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1001.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1001.cpp");
    deps.insert_metadata("P1001", Some(sample_metadata("P1001")));
    let submission = sample_submission(42, "AC");
    deps.insert_submissions("P1001", vec![submission.clone()]);
    let ui = FakeUi::default();

    run_record_bind_with(
        workspace.path().to_path_buf(),
        workspace.path().join("P1001.cpp"),
        Some(42),
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let commits = deps.created_commits();
    assert_eq!(commits.len(), 1);
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
async fn record_bind_falls_back_to_ui_when_submission_id_missing() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1002.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1002.cpp");
    deps.insert_metadata("P1002", Some(sample_metadata("P1002")));
    let submission = sample_submission(7, "WA");
    deps.insert_submissions("P1002", vec![submission.clone()]);
    let ui = FakeUi::with_record_submission(Some(submission));

    run_record_bind_with(
        workspace.path().to_path_buf(),
        workspace.path().join("P1002.cpp"),
        None,
        &deps,
        &ui,
    )
    .await
    .unwrap();

    assert_eq!(deps.created_commits().len(), 1);
}

#[tokio::test]
async fn record_bind_rejects_unknown_cli_submission_id() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1003.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1003.cpp");
    deps.insert_metadata("P1003", Some(sample_metadata("P1003")));
    deps.insert_submissions("P1003", vec![sample_submission(1, "AC")]);
    let ui = FakeUi::default();

    let error = run_record_bind_with(
        workspace.path().to_path_buf(),
        workspace.path().join("P1003.cpp"),
        Some(999),
        &deps,
        &ui,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("不属于题目 P1003"));
}
