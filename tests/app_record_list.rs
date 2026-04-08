mod support;

use aclog::{
    app::{render_record_list_output, run_record_list_with},
    domain::record::FileRecordSummary,
};

use chrono::{FixedOffset, TimeZone};
use support::{FakeDeps, workspace_with_config};

#[tokio::test]
async fn record_list_filters_untracked_files_and_writes_output() {
    let workspace = workspace_with_config();
    let deps = FakeDeps::default();
    deps.set_commit_descriptions(vec![
        (
            "rev-a".to_string(),
            "solve(P1001): title\n\nSubmission-ID: 1\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: tracked/P1001.cpp".to_string(),
        ),
        (
            "rev-b".to_string(),
            "solve(P1002): title\n\nSubmission-ID: 2\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: stale/P1002.cpp".to_string(),
        ),
    ]);
    deps.track_file("tracked/P1001.cpp");

    run_record_list_with(workspace.path().to_path_buf(), &deps)
        .await
        .unwrap();

    let output = deps.outputs().join("");
    assert!(output.contains("tracked/P1001.cpp"));
    assert!(!output.contains("stale/P1002.cpp"));
}

#[test]
fn record_list_render_output_handles_empty_and_non_empty_views() {
    let empty = render_record_list_output(&[]);
    assert_eq!(empty, "当前工作区还没有已记录的解法文件\n");

    let records = vec![FileRecordSummary {
        revision: "rev".to_string(),
        problem_id: "P1001".to_string(),
        title: "A+B Problem".to_string(),
        file_name: "P1001.cpp".to_string(),
        verdict: "AC".to_string(),
        difficulty: "入门".to_string(),
        submission_id: Some(1),
        submission_time: Some(
            FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 2, 3, 4, 0)
                .single()
                .unwrap(),
        ),
    }];
    let rendered = render_record_list_output(&records);
    assert!(rendered.contains("FILE\tPID\tVERDICT"));
    assert!(rendered.contains("P1001.cpp"));
}
