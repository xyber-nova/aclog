mod support;

use aclog::app::{TrainingFieldsPatch, run_record_edit_with, run_record_show_with};

use support::{FakeDeps, workspace_with_config, write_workspace_file};

#[tokio::test]
async fn record_show_outputs_latest_record_detail() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1001.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1001.cpp");
    deps.set_commit_descriptions(vec![(
        "rev-a".to_string(),
        "solve(P1001): title\n\nVerdict: AC\nDifficulty: 入门\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: P1001.cpp\nNote: 记得复习".to_string(),
    )]);

    run_record_show_with(
        workspace.path().to_path_buf(),
        workspace.path().join("P1001.cpp"),
        None,
        false,
        &deps,
    )
    .await
    .unwrap();

    let output = deps.outputs().join("");
    assert!(output.contains("版本: rev-a"));
    assert!(output.contains("笔记: 记得复习"));
}

#[tokio::test]
async fn record_edit_rewrites_only_training_fields() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "P1002.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("P1002.cpp");
    deps.set_commit_descriptions(vec![(
        "rev-b".to_string(),
        "solve(P1002): title\n\nVerdict: WA\nScore: 60\nTime: 12ms\nMemory: 1.5MB\nDifficulty: 入门\nSource: Luogu\nSubmission-ID: 2\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: P1002.cpp\nNote: 老备注".to_string(),
    )]);

    run_record_edit_with(
        workspace.path().to_path_buf(),
        workspace.path().join("P1002.cpp"),
        None,
        TrainingFieldsPatch {
            note: Some("".to_string()),
            insight: Some("补上二分边界".to_string()),
            ..TrainingFieldsPatch::default()
        },
        &deps,
    )
    .await
    .unwrap();

    let rewritten = deps.rewritten_descriptions();
    assert_eq!(rewritten.len(), 1);
    assert_eq!(rewritten[0].0, "rev-b");
    assert!(rewritten[0].1.contains("Insight: 补上二分边界"));
    assert!(rewritten[0].1.contains("Verdict: WA"));
    assert!(!rewritten[0].1.contains("Note: 老备注"));
}

#[tokio::test]
async fn record_show_rejects_non_luogu_problem_files() {
    let workspace = workspace_with_config();
    write_workspace_file(workspace.path(), "CF1234A.cpp", "int main() {}");

    let deps = FakeDeps::default();
    deps.track_file("CF1234A.cpp");

    let error = run_record_show_with(
        workspace.path().to_path_buf(),
        workspace.path().join("CF1234A.cpp"),
        None,
        false,
        &deps,
    )
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("受支持的题目标识（当前支持 Luogu / AtCoder）")
    );
}
