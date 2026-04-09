use std::{path::Path, process::Command};

use color_eyre::{
    Result,
    eyre::{WrapErr, eyre},
};
use tracing::{debug, info, instrument};

#[instrument(level = "info", skip_all, fields(workspace = %workspace_root.display()))]
pub async fn init_repo(workspace_root: &Path) -> Result<()> {
    if workspace_root.join(".jj").exists() {
        info!("jj 工作区已存在");
        return Ok(());
    }
    info!("正在初始化同目录 jj 仓库");
    run_jj_in_dir(workspace_root, "git init --colocate .").wrap_err("初始化 jj 工作区失败")?;
    info!("jj 工作区初始化完成");
    Ok(())
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace_root.display(), commits = commits.len()))]
pub(crate) async fn create_commits_for_files(
    workspace_root: &Path,
    commits: &[(String, String)],
) -> Result<()> {
    if commits.is_empty() {
        info!("没有需要创建的提交");
        return Ok(());
    }
    for (file, message) in commits {
        info!(file, "正在创建 jj 提交");
        debug!(file, %message, "提交消息详情");
        let command = format!(
            "commit --no-pager -m {} {}",
            shell_quote(message),
            shell_quote(file)
        );
        run_jj(workspace_root, &command).wrap_err_with(|| format!("为文件 {file} 创建提交失败"))?;
    }
    Ok(())
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace_root.display(), revision))]
pub(crate) async fn rewrite_commit_description(
    workspace_root: &Path,
    revision: &str,
    message: &str,
) -> Result<()> {
    let command = format!(
        "describe --no-pager -r {} -m {}",
        shell_quote(revision),
        shell_quote(message)
    );
    run_jj(workspace_root, &command).wrap_err_with(|| format!("重写提交 {revision} 的描述失败"))?;
    Ok(())
}

fn run_jj(workspace_root: &Path, args: &str) -> Result<()> {
    let command = format!("jj -R {} {}", shell_quote_path(workspace_root), args);
    run_jj_command(workspace_root, &command).map(|_| ())
}

fn run_jj_in_dir(workspace_root: &Path, args: &str) -> Result<()> {
    let command = format!("jj {}", args);
    run_jj_command(workspace_root, &command).map(|_| ())
}

fn run_jj_command(workspace_root: &Path, command: &str) -> Result<String> {
    debug!(
        workspace = %workspace_root.display(),
        %command,
        "正在执行 jj 命令"
    );
    let output = Command::new("zsh")
        .arg("-lc")
        .arg(command)
        .current_dir(workspace_root)
        .output()
        .wrap_err("启动 jj 命令失败")?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        debug!(
            workspace = %workspace_root.display(),
            %command,
            stdout = %stdout,
            stderr = %stderr,
            "jj 命令执行完成"
        );
        Ok(stdout)
    } else {
        Err(eyre!(
            "jj 命令执行失败：\n命令：{}\n标准输出：\n{}\n标准错误：\n{}",
            command,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

fn shell_quote_path(path: &Path) -> String {
    shell_quote(&path.display().to_string())
}
