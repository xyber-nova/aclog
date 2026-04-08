use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn aclog_init_creates_workspace_layout() {
    let workspace = tempdir().unwrap();

    Command::cargo_bin("aclog")
        .unwrap()
        .current_dir(workspace.path())
        .arg("init")
        .assert()
        .success();

    assert!(workspace.path().join(".aclog/config.toml").exists());
    assert!(workspace.path().join(".jj").exists());
}

#[test]
fn aclog_record_list_smoke_works_after_init() {
    let workspace = tempdir().unwrap();

    Command::cargo_bin("aclog")
        .unwrap()
        .current_dir(workspace.path())
        .arg("init")
        .assert()
        .success();

    Command::cargo_bin("aclog")
        .unwrap()
        .current_dir(workspace.path())
        .args(["record", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("当前工作区还没有已记录的解法文件"));
}

#[test]
fn aclog_stats_reports_missing_config_values() {
    let workspace = tempdir().unwrap();

    Command::cargo_bin("aclog")
        .unwrap()
        .current_dir(workspace.path())
        .arg("init")
        .assert()
        .success();

    Command::cargo_bin("aclog")
        .unwrap()
        .current_dir(workspace.path())
        .arg("stats")
        .assert()
        .failure()
        .stderr(predicate::str::contains("user.luogu_uid"));
}

#[test]
fn aclog_record_bind_reports_missing_file() {
    let workspace = tempdir().unwrap();
    fs::create_dir_all(workspace.path()).unwrap();

    Command::cargo_bin("aclog")
        .unwrap()
        .current_dir(workspace.path())
        .args(["record", "bind", "P9999.cpp"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("未找到 jj 工作区").or(predicate::str::contains("不存在")),
        );
}
