mod support;

use aclog::{
    app::{SyncOptions, run_sync_with_full_options, run_sync_with_options},
    commit_format::build_solve_commit_message,
    domain::record::SyncSelection,
    ui::interaction::SyncBatchDetailAction,
};

use support::{
    FakeDeps, FakeUi, active_change, deleted_change, sample_atcoder_metadata,
    sample_atcoder_submission, sample_metadata, sample_submission, workspace_with_config,
};

#[tokio::test]
async fn sync_skips_unparseable_files_and_commits_selected_submission() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("notes.txt"), active_change("P1001.cpp")]);
    deps.insert_metadata("luogu:P1001", Some(sample_metadata("luogu:P1001")));
    let mut submission = sample_submission(42, "AC");
    submission.problem_id = Some("luogu:P1001".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1001", vec![submission.clone()]);
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
            "luogu:P1001",
            "P1001.cpp",
            Some(&sample_metadata("luogu:P1001")),
            &submission
        )
    );
}

#[tokio::test]
async fn sync_creates_chore_commit_when_user_selects_chore() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P1002.cpp")]);
    deps.insert_metadata("luogu:P1002", Some(sample_metadata("luogu:P1002")));
    deps.insert_submissions("luogu:P1002", vec![]);
    let ui = FakeUi::with_submission_selection(SyncSelection::Chore);

    run_sync_with_options(workspace.path().to_path_buf(), false, &deps, &ui)
        .await
        .unwrap();

    let commits = deps.created_commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].0, "P1002.cpp");
    assert!(commits[0].1.starts_with("chore(luogu:P1002):"));
}

#[tokio::test]
async fn sync_creates_delete_commit_for_deleted_files() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![deleted_change("P1003.cpp")]);
    deps.insert_metadata("luogu:P1003", Some(sample_metadata("luogu:P1003")));
    let ui = FakeUi::with_delete_confirmation(SyncSelection::Delete);

    run_sync_with_options(workspace.path().to_path_buf(), false, &deps, &ui)
        .await
        .unwrap();

    let commits = deps.created_commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].0, "P1003.cpp");
    assert!(commits[0].1.starts_with("remove(luogu:P1003):"));
}

#[tokio::test]
async fn sync_skip_produces_no_commit() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P1004.cpp")]);
    deps.insert_metadata("luogu:P1004", Some(sample_metadata("luogu:P1004")));
    let mut submission = sample_submission(1, "WA");
    submission.problem_id = Some("luogu:P1004".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1004", vec![submission]);
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
    deps.insert_metadata("luogu:P1005", Some(sample_metadata("luogu:P1005")));
    let mut ac = sample_submission(10, "AC");
    ac.problem_id = Some("luogu:P1005".to_string());
    ac.provider = aclog::problem::ProblemProvider::Luogu;
    let mut wa = sample_submission(9, "WA");
    wa.problem_id = Some("luogu:P1005".to_string());
    wa.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1005", vec![ac, wa]);
    let ui = FakeUi::default();

    run_sync_with_options(workspace.path().to_path_buf(), true, &deps, &ui)
        .await
        .unwrap();

    assert!(deps.created_commits().is_empty());
    let output = deps.outputs().join("");
    assert!(output.contains("P1005.cpp"));
    assert!(output.contains("等待选择提交记录"));
    assert!(!output.contains("notes.txt"));
}

#[tokio::test]
async fn sync_filters_out_non_luogu_problem_prefixes() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![
        active_change("CF1234A.cpp"),
        active_change("P1006.cpp"),
    ]);
    deps.insert_metadata("luogu:P1006", Some(sample_metadata("luogu:P1006")));
    let mut submission = sample_submission(100, "AC");
    submission.problem_id = Some("luogu:P1006".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1006", vec![submission]);
    let ui = FakeUi::with_submission_selection(SyncSelection::Skip);

    run_sync_with_options(workspace.path().to_path_buf(), true, &deps, &ui)
        .await
        .unwrap();

    let output = deps.outputs().join("");
    assert!(output.contains("P1006.cpp"));
    assert!(!output.contains("CF1234A.cpp"));
}

#[tokio::test]
async fn sync_dry_run_handles_mixed_luogu_and_atcoder_files() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![
        active_change("P1010.cpp"),
        active_change("abc350_a.cpp"),
        active_change("notes.txt"),
    ]);
    deps.insert_metadata("luogu:P1010", Some(sample_metadata("luogu:P1010")));
    deps.insert_metadata(
        "atcoder:abc350_a",
        Some(sample_atcoder_metadata("atcoder:abc350_a", Some("ABC350"))),
    );
    let luogu = sample_submission(101, "AC");
    let atcoder = sample_atcoder_submission(350001, "atcoder:abc350_a", "AC");
    deps.insert_submissions("luogu:P1010", vec![luogu]);
    deps.insert_submissions("atcoder:abc350_a", vec![atcoder]);

    run_sync_with_options(
        workspace.path().to_path_buf(),
        true,
        &deps,
        &FakeUi::default(),
    )
    .await
    .unwrap();

    let output = deps.outputs().join("");
    assert!(output.contains("P1010.cpp"));
    assert!(output.contains("abc350_a.cpp"));
    assert!(!output.contains("notes.txt"));
    assert_eq!(deps.submission_fetch_count("luogu:P1010"), 1);
    assert_eq!(deps.submission_fetch_count("atcoder:abc350_a"), 1);
}

#[tokio::test]
async fn sync_prefetches_submissions_in_parallel() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![
        active_change("P1100.cpp"),
        active_change("P1101.cpp"),
        active_change("P1102.cpp"),
    ]);
    deps.insert_metadata("luogu:P1100", Some(sample_metadata("luogu:P1100")));
    deps.insert_metadata("luogu:P1101", Some(sample_metadata("luogu:P1101")));
    deps.insert_metadata("luogu:P1102", Some(sample_metadata("luogu:P1102")));
    let mut s1100 = sample_submission(11, "AC");
    s1100.problem_id = Some("luogu:P1100".to_string());
    s1100.provider = aclog::problem::ProblemProvider::Luogu;
    let mut s1101 = sample_submission(12, "AC");
    s1101.problem_id = Some("luogu:P1101".to_string());
    s1101.provider = aclog::problem::ProblemProvider::Luogu;
    let mut s1102 = sample_submission(13, "AC");
    s1102.problem_id = Some("luogu:P1102".to_string());
    s1102.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1100", vec![s1100]);
    deps.insert_submissions("luogu:P1101", vec![s1101]);
    deps.insert_submissions("luogu:P1102", vec![s1102]);
    deps.configure_submission_fetch_barrier(3);

    run_sync_with_options(
        workspace.path().to_path_buf(),
        true,
        &deps,
        &FakeUi::default(),
    )
    .await
    .unwrap();

    assert!(deps.max_submission_fetch_in_flight() > 1);
    assert_eq!(deps.submission_fetch_count("luogu:P1100"), 1);
    assert_eq!(deps.submission_fetch_count("luogu:P1101"), 1);
    assert_eq!(deps.submission_fetch_count("luogu:P1102"), 1);
}

#[tokio::test]
async fn sync_reuses_prefetched_submissions_in_detail_and_commit_stages() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P1110.cpp")]);
    deps.insert_metadata("luogu:P1110", Some(sample_metadata("luogu:P1110")));
    let mut submission = sample_submission(111, "AC");
    submission.problem_id = Some("luogu:P1110".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P1110", vec![submission.clone()]);
    let ui = FakeUi::with_submission_selection(SyncSelection::Submission(submission));

    run_sync_with_full_options(
        workspace.path().to_path_buf(),
        SyncOptions::default(),
        &deps,
        &ui,
    )
    .await
    .unwrap();

    assert_eq!(deps.submission_fetch_count("luogu:P1110"), 1);
}

#[tokio::test]
async fn sync_resume_drops_legacy_non_luogu_session_items() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P2102.cpp")]);
    deps.insert_metadata("luogu:P2102", Some(sample_metadata("luogu:P2102")));
    let mut submission = sample_submission(92, "AC");
    submission.problem_id = Some("luogu:P2102".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P2102", vec![submission]);

    std::fs::write(
        workspace.path().join(".aclog/sync-session.toml"),
        r#"
created_at = "2024-01-01T00:00:00+08:00"

[[items]]
file = "CF1234A.cpp"
problem_id = "CF1234A"
kind = "Active"
status = "Pending"
submissions = 1
default_submission_id = 77
warnings = []
"#,
    )
    .unwrap();

    run_sync_with_options(
        workspace.path().to_path_buf(),
        true,
        &deps,
        &FakeUi::default(),
    )
    .await
    .unwrap();

    let output = deps.outputs().join("");
    assert!(!output.contains("CF1234A.cpp"));
    assert!(output.contains("P2102.cpp"));
}

#[tokio::test]
async fn sync_resume_uses_saved_session_and_clears_it_after_commit() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P2001.cpp")]);
    deps.insert_metadata("luogu:P2001", Some(sample_metadata("luogu:P2001")));
    let mut submission = sample_submission(66, "AC");
    submission.problem_id = Some("luogu:P2001".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P2001", vec![submission.clone()]);

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
async fn sync_quit_from_resume_prompt_preserves_saved_session() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P2100.cpp")]);
    deps.insert_metadata("luogu:P2100", Some(sample_metadata("luogu:P2100")));
    let mut submission = sample_submission(90, "AC");
    submission.problem_id = Some("luogu:P2100".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P2100", vec![submission]);

    std::fs::write(
        workspace.path().join(".aclog/sync-session.toml"),
        r#"
created_at = "2024-01-01T00:00:00+08:00"

[[items]]
file = "P2100.cpp"
problem_id = "P2100"
kind = "Active"
status = "Pending"
submissions = 1
default_submission_id = 90
warnings = []
"#,
    )
    .unwrap();

    let ui = FakeUi {
        sync_session_choice: std::sync::Mutex::new(Some(
            aclog::domain::sync_batch::SyncSessionChoice::Quit,
        )),
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

    assert!(workspace.path().join(".aclog/sync-session.toml").exists());
    assert!(deps.created_commits().is_empty());
}

#[tokio::test]
async fn sync_quit_from_detail_preserves_saved_session() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P2101.cpp")]);
    deps.insert_metadata("luogu:P2101", Some(sample_metadata("luogu:P2101")));
    let mut submission = sample_submission(91, "WA");
    submission.problem_id = Some("luogu:P2101".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P2101", vec![submission]);

    let ui = FakeUi {
        sync_batch_detail_action: std::sync::Mutex::new(Some(SyncBatchDetailAction::Quit)),
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

    assert!(workspace.path().join(".aclog/sync-session.toml").exists());
    assert!(deps.created_commits().is_empty());
}

#[tokio::test]
async fn sync_duplicate_submission_requires_explicit_second_confirmation() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_changed_files(vec![active_change("P2002.cpp")]);
    deps.insert_metadata("luogu:P2002", Some(sample_metadata("luogu:P2002")));
    let mut submission = sample_submission(77, "AC");
    submission.problem_id = Some("luogu:P2002".to_string());
    submission.provider = aclog::problem::ProblemProvider::Luogu;
    deps.insert_submissions("luogu:P2002", vec![submission.clone()]);
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
