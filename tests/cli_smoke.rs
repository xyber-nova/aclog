use std::fs;
#[cfg(unix)]
use std::process::Command as ProcessCommand;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn aclog_help_still_prints_cli_usage() {
    Command::cargo_bin("aclog")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "用于在本地 jj 工作区中记录、同步、浏览和复盘算法训练过程的 CLI 工具。",
        ))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("stats"));
}

#[cfg(unix)]
#[test]
fn aclog_without_args_opens_home_workbench() {
    if ProcessCommand::new("script")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping PTY smoke because `script` is unavailable");
        return;
    }

    let workspace = tempdir().unwrap();

    Command::cargo_bin("aclog")
        .unwrap()
        .current_dir(workspace.path())
        .arg("init")
        .assert()
        .success();

    let output = ProcessCommand::new("sh")
        .current_dir(workspace.path())
        .env("ACLOG_BIN", env!("CARGO_BIN_EXE_aclog"))
        .env("TERM", "xterm-256color")
        .env("COLUMNS", "120")
        .env("LINES", "40")
        .arg("-lc")
        .arg("(sleep 0.05; printf q) | script -qec \"$ACLOG_BIN\" /dev/null")
        .output()
        .expect("home workbench smoke should run under a PTY");

    assert!(
        output.status.success(),
        "aclog without args should open and quit the home workbench successfully\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let transcript = String::from_utf8_lossy(&output.stdout);
    assert!(
        transcript.contains("\u{1b}[?1049h"),
        "expected home workbench smoke to enter alternate screen, got:\n{transcript}"
    );
    assert!(
        transcript.contains("\u{1b}[?1049l"),
        "expected home workbench smoke to leave alternate screen, got:\n{transcript}"
    );
}

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
