mod support;

use aclog::{app::run_record_bind_with, commit_format::build_solve_commit_message};

use support::{
    FakeDeps, FakeUi, sample_atcoder_metadata, sample_atcoder_submission, sample_metadata,
    sample_submission, workspace_with_config, write_workspace_file,
};

#[tokio::test]
async fn record_bind_uses_cli_submission_id_without_ui() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1001.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1001.cpp");
    deps.insert_metadata("luogu:P1001", Some(sample_metadata("luogu:P1001")));
    let mut submission = sample_submission(42, "AC");
    submission.problem_id = Some("luogu:P1001".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1001", vec![submission.clone()]);
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
            "luogu:P1001",
            "P1001.cpp",
            Some(&sample_metadata("luogu:P1001")),
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
    deps.insert_metadata("luogu:P1002", Some(sample_metadata("luogu:P1002")));
    let mut submission = sample_submission(7, "WA");
    submission.problem_id = Some("luogu:P1002".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1002", vec![submission.clone()]);
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
    deps.insert_metadata("luogu:P1003", Some(sample_metadata("luogu:P1003")));
    let mut submission = sample_submission(1, "AC");
    submission.problem_id = Some("luogu:P1003".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1003", vec![submission]);
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

#[tokio::test]
async fn record_bind_rejects_non_luogu_problem_files() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "CF1234A.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("CF1234A.cpp");
    let ui = FakeUi::default();

    let error = run_record_bind_with(
        workspace.path().to_path_buf(),
        workspace.path().join("CF1234A.cpp"),
        None,
        &deps,
        &ui,
    )
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("受支持的题目标识（当前支持 Luogu / AtCoder）")
    );
}

#[tokio::test]
async fn record_bind_passes_atcoder_source_and_contest_context_to_selector() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "abc350_a.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("abc350_a.cpp");
    deps.insert_metadata(
        "atcoder:abc350_a",
        Some(sample_atcoder_metadata("atcoder:abc350_a", Some("ABC350"))),
    );
    let submission = sample_atcoder_submission(350042, "atcoder:abc350_a", "AC");
    deps.insert_submissions("atcoder:abc350_a", vec![submission.clone()]);
    let ui = FakeUi::with_record_submission(Some(submission));

    run_record_bind_with(
        workspace.path().to_path_buf(),
        workspace.path().join("abc350_a.cpp"),
        None,
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let requests = ui.record_submission_requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "atcoder:abc350_a");
    assert_eq!(
        requests[0]
            .1
            .as_ref()
            .and_then(|item| item.source.as_deref()),
        Some("AtCoder")
    );
    assert_eq!(
        requests[0]
            .1
            .as_ref()
            .and_then(|item| item.contest.as_deref()),
        Some("ABC350")
    );
    assert_eq!(requests[0].2.len(), 1);
}
