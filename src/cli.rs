use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use color_eyre::Result;
use color_eyre::eyre::{WrapErr, eyre};
use tracing::{debug, info, instrument};

use crate::config::AclogPaths;
use crate::models::{
    FileRecordSummary, HistoricalSolveRecord, ProblemMetadata, SubmissionRecord, SyncSelection,
};
use crate::vcs::ProblemFileChangeKind;

#[derive(Debug, Parser)]
#[command(
    name = "aclog",
    version,
    about = "OI training log tool for jj repositories"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
    Sync {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
    Stats {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
    Record {
        #[command(subcommand)]
        command: RecordCommands,
    },
}

#[derive(Debug, Subcommand)]
enum RecordCommands {
    Bind {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        submission_id: Option<u64>,
    },
    Rebind {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long = "record-rev")]
        record_rev: Option<String>,
        #[arg(long)]
        submission_id: Option<u64>,
    },
    List {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    run_command(cli.command).await
}

async fn run_command(command: Commands) -> Result<()> {
    match command {
        Commands::Init { workspace } => run_init(workspace).await,
        Commands::Sync { workspace } => run_sync(workspace).await,
        Commands::Stats { workspace } => run_stats(workspace).await,
        Commands::Record { command } => run_record_command(command).await,
    }
}

async fn run_record_command(command: RecordCommands) -> Result<()> {
    match command {
        RecordCommands::Bind {
            file,
            workspace,
            submission_id,
        } => run_record_bind(workspace, file, submission_id).await,
        RecordCommands::Rebind {
            file,
            workspace,
            record_rev,
            submission_id,
        } => run_record_rebind(workspace, file, record_rev, submission_id).await,
        RecordCommands::List { workspace } => run_record_list(workspace).await,
    }
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
async fn run_init(workspace: PathBuf) -> Result<()> {
    info!("开始初始化");
    crate::config::init_workspace(&workspace).await?;
    info!("初始化完成");
    Ok(())
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
async fn run_sync(workspace: PathBuf) -> Result<()> {
    info!("开始同步");

    let paths = AclogPaths::new(workspace)?;
    let config = crate::config::load_config(&paths)?;
    crate::vcs::ensure_jj_workspace(&paths.workspace_root)?;

    let changed_files = crate::vcs::collect_changed_problem_files(&paths.workspace_root).await?;
    info!(changed_files = changed_files.len(), "已收集变更文件");
    debug!(?changed_files, "变更文件详情");

    let mut planned_commits = Vec::new();
    for change in changed_files {
        let file = change.path;
        let Some(problem_id) = crate::problem::extract_problem_id(&file) else {
            debug!(file, kind = ?change.kind, "跳过无法识别题号的文件");
            continue;
        };
        info!(file, problem_id, kind = ?change.kind, "处理变更题目文件");

        let metadata = crate::api::resolve_problem_metadata(&config, &paths, &problem_id).await?;
        let selection = if should_fetch_submissions(change.kind) {
            let submissions =
                crate::api::fetch_problem_submissions(&config, &paths, &problem_id).await?;
            info!(
                file,
                problem_id,
                kind = ?change.kind,
                submissions = submissions.len(),
                has_metadata = metadata.is_some(),
                "已获取同步上下文"
            );
            crate::tui::select_submission(&problem_id, metadata.as_ref(), &submissions)?
        } else {
            info!(
                file,
                problem_id,
                kind = ?change.kind,
                has_metadata = metadata.is_some(),
                "已获取删除确认上下文"
            );
            crate::tui::confirm_deleted_file(&problem_id, metadata.as_ref())?
        };
        info!(
            file,
            problem_id,
            selection = selection_kind(&selection),
            "同步选择完成"
        );
        if let Some(commit) = planned_commit(&problem_id, &file, metadata.as_ref(), &selection) {
            planned_commits.push(commit);
        }
    }

    info!(planned_commits = planned_commits.len(), "已生成提交计划");
    if !planned_commits.is_empty() {
        crate::vcs::create_commits_for_files(&paths.workspace_root, &planned_commits).await?;
    }

    info!("同步完成");
    Ok(())
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
async fn run_stats(workspace: PathBuf) -> Result<()> {
    info!("开始统计");

    let paths = AclogPaths::new(workspace)?;
    crate::vcs::ensure_jj_workspace(&paths.workspace_root)?;
    let config = crate::config::load_config(&paths)?;

    let messages = crate::vcs::collect_solve_commit_messages(&paths.workspace_root).await?;
    let records = crate::models::parse_solve_records(&messages);
    let algorithm_tag_names = crate::api::load_algorithm_tag_names(&config, &paths).await?;
    let summary = crate::models::summarize_solve_records(&records, Some(&algorithm_tag_names));

    crate::tui::show_stats(&paths.workspace_root, &summary)?;

    info!(
        total_solve_records = summary.total_solve_records,
        unique_problem_count = summary.unique_problem_count,
        "统计完成"
    );
    Ok(())
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display(), file = %file.display()))]
async fn run_record_bind(
    workspace: PathBuf,
    file: PathBuf,
    submission_id: Option<u64>,
) -> Result<()> {
    info!("开始手工绑定记录");

    let paths = AclogPaths::new(workspace)?;
    crate::vcs::ensure_jj_workspace(&paths.workspace_root)?;
    let target = resolve_solution_file_target(&paths, &file).await?;
    let needs_submission_choice = submission_selection_plan(submission_id);
    let config = crate::config::load_config(&paths)?;

    let metadata = crate::api::resolve_problem_metadata(&config, &paths, &target.problem_id).await?;
    let submissions =
        crate::api::fetch_problem_submissions(&config, &paths, &target.problem_id).await?;
    let record = select_submission_for_record(
        &target.problem_id,
        metadata.as_ref(),
        &submissions,
        submission_id,
    )?;
    debug!(needs_submission_choice, "record bind 交互计划已确定");
    let message = crate::models::build_solve_commit_message(
        &target.problem_id,
        &target.repo_relative_path,
        metadata.as_ref(),
        &record,
    );
    crate::vcs::create_commits_for_files(
        &paths.workspace_root,
        &[(target.repo_relative_path.clone(), message)],
    )
    .await?;

    info!(problem_id = target.problem_id, "手工绑定完成");
    Ok(())
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display(), file = %file.display()))]
async fn run_record_rebind(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    submission_id: Option<u64>,
) -> Result<()> {
    info!("开始重绑记录");

    let paths = AclogPaths::new(workspace)?;
    crate::vcs::ensure_jj_workspace(&paths.workspace_root)?;
    let target = resolve_solution_file_target(&paths, &file).await?;
    let selection_plan = rebind_selection_plan(record_rev.as_deref(), submission_id);
    let history_entries = crate::vcs::collect_commit_descriptions(&paths.workspace_root).await?;
    let history_records = crate::models::parse_historical_solve_records(&history_entries);
    let candidates = history_records_for_file(
        &history_records,
        &target.repo_relative_path,
        &target.problem_id,
    );
    let selected_record =
        select_record_for_rebind(&paths, &target, &candidates, record_rev.as_deref()).await?;

    let config = crate::config::load_config(&paths)?;
    let metadata = crate::api::resolve_problem_metadata(&config, &paths, &target.problem_id).await?;
    let submissions =
        crate::api::fetch_problem_submissions(&config, &paths, &target.problem_id).await?;
    let record = select_submission_for_record(
        &target.problem_id,
        metadata.as_ref(),
        &submissions,
        submission_id,
    )?;
    debug!(?selection_plan, "record rebind 交互计划已确定");
    let message = crate::models::build_solve_commit_message(
        &target.problem_id,
        &target.repo_relative_path,
        metadata.as_ref(),
        &record,
    );
    crate::vcs::rewrite_commit_description(
        &paths.workspace_root,
        &selected_record.revision,
        &message,
    )
    .await?;

    info!(problem_id = target.problem_id, revision = selected_record.revision, "重绑完成");
    Ok(())
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
async fn run_record_list(workspace: PathBuf) -> Result<()> {
    info!("开始列出已记录文件");

    let paths = AclogPaths::new(workspace)?;
    crate::vcs::ensure_jj_workspace(&paths.workspace_root)?;
    let history_entries = crate::vcs::collect_commit_descriptions(&paths.workspace_root).await?;
    let history_records = crate::models::parse_historical_solve_records(&history_entries);
    let summaries = crate::models::latest_records_by_file(&history_records);
    let mut tracked_summaries = Vec::with_capacity(summaries.len());
    for summary in summaries {
        if crate::vcs::is_tracked_file(&paths.workspace_root, &summary.file_name).await? {
            tracked_summaries.push(summary);
        }
    }
    print_record_list(&tracked_summaries);

    info!(records = tracked_summaries.len(), "已输出记录列表");
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SolutionFileTarget {
    problem_id: String,
    repo_relative_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RebindSelectionPlan {
    needs_record_choice: bool,
    needs_submission_choice: bool,
}

async fn resolve_solution_file_target(paths: &AclogPaths, file: &Path) -> Result<SolutionFileTarget> {
    let absolute_path = if file.is_absolute() {
        file.to_path_buf()
    } else {
        paths.workspace_root.join(file)
    };
    if !absolute_path.exists() {
        return Err(eyre!("文件 {} 不存在", absolute_path.display()));
    }
    if !absolute_path.is_file() {
        return Err(eyre!("{} 不是普通文件", absolute_path.display()));
    }
    let canonical_path = absolute_path
        .canonicalize()
        .wrap_err_with(|| format!("解析文件路径 {} 失败", absolute_path.display()))?;
    let repo_relative_path = canonical_path
        .strip_prefix(&paths.workspace_root)
        .wrap_err_with(|| format!("文件 {} 不在当前工作区内", canonical_path.display()))?
        .to_string_lossy()
        .replace('\\', "/");
    let file_name = canonical_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| eyre!("无法解析文件名 {}", canonical_path.display()))?;
    let problem_id = crate::problem::extract_problem_id(file_name)
        .ok_or_else(|| eyre!("无法从文件名 {} 提取题号", file_name))?;
    if !crate::vcs::is_tracked_file(&paths.workspace_root, &repo_relative_path).await? {
        return Err(eyre!("文件 {} 未被当前 jj 工作区跟踪", repo_relative_path));
    }
    Ok(SolutionFileTarget {
        problem_id,
        repo_relative_path,
    })
}

fn submission_selection_plan(submission_id: Option<u64>) -> bool {
    submission_id.is_none()
}

fn rebind_selection_plan(record_rev: Option<&str>, submission_id: Option<u64>) -> RebindSelectionPlan {
    RebindSelectionPlan {
        needs_record_choice: record_rev.is_none(),
        needs_submission_choice: submission_id.is_none(),
    }
}

fn history_records_for_file(
    records: &[HistoricalSolveRecord],
    repo_relative_path: &str,
    problem_id: &str,
) -> Vec<HistoricalSolveRecord> {
    records
        .iter()
        .filter(|entry| {
            entry.record.file_name == repo_relative_path && entry.record.problem_id == problem_id
        })
        .cloned()
        .collect()
}

fn select_submission_for_record(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
    submission_id: Option<u64>,
) -> Result<SubmissionRecord> {
    if let Some(submission_id) = submission_id {
        return submissions
            .iter()
            .find(|record| record.submission_id == submission_id)
            .cloned()
            .ok_or_else(|| eyre!("提交记录 {} 不属于题目 {}", submission_id, problem_id));
    }

    crate::tui::select_record_submission(problem_id, metadata, submissions)?
        .ok_or_else(|| eyre!("已取消选择提交记录"))
}

async fn select_record_for_rebind(
    paths: &AclogPaths,
    target: &SolutionFileTarget,
    candidates: &[HistoricalSolveRecord],
    record_rev: Option<&str>,
) -> Result<HistoricalSolveRecord> {
    if candidates.is_empty() {
        return Err(eyre!(
            "文件 {} 当前没有可重绑的记录",
            target.repo_relative_path
        ));
    }

    if let Some(record_rev) = record_rev {
        let revision = crate::vcs::resolve_single_commit_id(&paths.workspace_root, record_rev).await?;
        let entry = candidates
            .iter()
            .find(|entry| entry.revision == revision)
            .cloned()
            .ok_or_else(|| {
                eyre!(
                    "`--record-rev` 指定的提交不是该文件 {} 的标准 solve 记录",
                    target.repo_relative_path
                )
            })?;
        if entry.record.problem_id != target.problem_id {
            return Err(eyre!(
                "`--record-rev` 指定的提交题号与文件 {} 不匹配",
                target.repo_relative_path
            ));
        }
        return Ok(entry);
    }

    crate::tui::select_record_to_rebind(
        &target.problem_id,
        &target.repo_relative_path,
        candidates,
    )?
    .ok_or_else(|| eyre!("已取消选择要重写的记录"))
}

fn print_record_list(records: &[FileRecordSummary]) {
    if records.is_empty() {
        println!("当前工作区还没有已记录的解法文件");
        return;
    }

    println!("FILE\tPID\tVERDICT\tDIFF\tSUBMISSION\tRECORDED-AT\tTITLE");
    for record in records {
        let submission_id = record
            .submission_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        let recorded_at = record
            .submission_time
            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            record.file_name,
            record.problem_id,
            record.verdict,
            record.difficulty,
            submission_id,
            recorded_at,
            record.title,
        );
    }
}

fn planned_commit(
    problem_id: &str,
    file: &str,
    metadata: Option<&ProblemMetadata>,
    selection: &SyncSelection,
) -> Option<(String, String)> {
    match selection {
        SyncSelection::Skip => None,
        _ => Some((
            file.to_string(),
            crate::models::build_commit_message(problem_id, file, metadata, selection),
        )),
    }
}

fn selection_kind(selection: &crate::models::SyncSelection) -> &'static str {
    match selection {
        crate::models::SyncSelection::Submission(_) => "submission",
        crate::models::SyncSelection::Chore => "chore",
        crate::models::SyncSelection::Delete => "delete",
        crate::models::SyncSelection::Skip => "skip",
    }
}

fn should_fetch_submissions(change_kind: ProblemFileChangeKind) -> bool {
    matches!(change_kind, ProblemFileChangeKind::Active)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::{FixedOffset, TimeZone};
    use clap::Parser as _;

    use super::{
        Cli, Commands, RecordCommands, RebindSelectionPlan, SolutionFileTarget,
        history_records_for_file, planned_commit, print_record_list, rebind_selection_plan,
        run_command, select_record_for_rebind, select_submission_for_record, selection_kind,
        should_fetch_submissions, submission_selection_plan,
    };
    use crate::config::AclogPaths;
    use crate::models::{HistoricalSolveRecord, ProblemMetadata, SolveRecord, SubmissionRecord, SyncSelection};
    use crate::vcs::ProblemFileChangeKind;

    fn sample_metadata() -> ProblemMetadata {
        ProblemMetadata {
            id: "P1001".to_string(),
            title: "A+B Problem".to_string(),
            difficulty: Some("入门".to_string()),
            tags: vec!["模拟".to_string()],
            source: Some("Luogu".to_string()),
            url: "https://www.luogu.com.cn/problem/P1001".to_string(),
            fetched_at: FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                .single()
                .unwrap(),
        }
    }

    fn sample_submission() -> SyncSelection {
        SyncSelection::Submission(SubmissionRecord {
            submission_id: 123456,
            submitter: "123456".to_string(),
            verdict: "AC".to_string(),
            score: Some(100),
            time_ms: Some(50),
            memory_mb: Some(1.2),
            submitted_at: Some(
                FixedOffset::east_opt(8 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                    .single()
                    .unwrap(),
            ),
        })
    }

    #[test]
    fn planned_commit_includes_submission_chore_delete_but_skips_skip() {
        let metadata = sample_metadata();

        let submission_commit =
            planned_commit("P1001", "P1001.cpp", Some(&metadata), &sample_submission());
        let chore_commit =
            planned_commit("P1001", "P1001.cpp", Some(&metadata), &SyncSelection::Chore);
        let delete_commit = planned_commit(
            "P1001",
            "P1001.cpp",
            Some(&metadata),
            &SyncSelection::Delete,
        );
        let skipped_commit =
            planned_commit("P1001", "P1001.cpp", Some(&metadata), &SyncSelection::Skip);

        assert!(submission_commit.is_some());
        assert!(chore_commit.is_some());
        assert!(delete_commit.is_some());
        assert!(skipped_commit.is_none());
    }

    #[test]
    fn selection_kind_matches_selection_variants() {
        assert_eq!(selection_kind(&sample_submission()), "submission");
        assert_eq!(selection_kind(&SyncSelection::Chore), "chore");
        assert_eq!(selection_kind(&SyncSelection::Delete), "delete");
        assert_eq!(selection_kind(&SyncSelection::Skip), "skip");
    }

    #[test]
    fn only_active_files_fetch_submissions() {
        assert!(should_fetch_submissions(ProblemFileChangeKind::Active));
        assert!(!should_fetch_submissions(ProblemFileChangeKind::Deleted));
    }

    #[test]
    fn cli_parses_stats_subcommand_with_default_workspace() {
        let cli = Cli::parse_from(["aclog", "stats"]);

        match cli.command {
            Commands::Stats { workspace } => assert_eq!(workspace, PathBuf::from(".")),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_bind_with_submission_id() {
        let cli = Cli::parse_from([
            "aclog",
            "record",
            "bind",
            "P1001.cpp",
            "--submission-id",
            "42",
        ]);

        match cli.command {
            Commands::Record {
                command:
                    RecordCommands::Bind {
                        file,
                        workspace,
                        submission_id,
                    },
            } => {
                assert_eq!(file, PathBuf::from("P1001.cpp"));
                assert_eq!(workspace, PathBuf::from("."));
                assert_eq!(submission_id, Some(42));
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_rebind_with_non_interactive_flags() {
        let cli = Cli::parse_from([
            "aclog",
            "record",
            "rebind",
            "P1001.cpp",
            "--record-rev",
            "abc123",
            "--submission-id",
            "42",
        ]);

        match cli.command {
            Commands::Record {
                command:
                    RecordCommands::Rebind {
                        file,
                        workspace,
                        record_rev,
                        submission_id,
                    },
            } => {
                assert_eq!(file, PathBuf::from("P1001.cpp"));
                assert_eq!(workspace, PathBuf::from("."));
                assert_eq!(record_rev.as_deref(), Some("abc123"));
                assert_eq!(submission_id, Some(42));
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_list_with_default_workspace() {
        let cli = Cli::parse_from(["aclog", "record", "list"]);

        match cli.command {
            Commands::Record {
                command: RecordCommands::List { workspace },
            } => assert_eq!(workspace, PathBuf::from(".")),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn submission_selection_plan_matches_cli_only_and_interactive_modes() {
        assert!(submission_selection_plan(None));
        assert!(!submission_selection_plan(Some(1)));
    }

    #[test]
    fn rebind_selection_plan_tracks_remaining_choices() {
        assert_eq!(
            rebind_selection_plan(None, None),
            RebindSelectionPlan {
                needs_record_choice: true,
                needs_submission_choice: true,
            }
        );
        assert_eq!(
            rebind_selection_plan(Some("abc"), None),
            RebindSelectionPlan {
                needs_record_choice: false,
                needs_submission_choice: true,
            }
        );
        assert_eq!(
            rebind_selection_plan(None, Some(1)),
            RebindSelectionPlan {
                needs_record_choice: true,
                needs_submission_choice: false,
            }
        );
        assert_eq!(
            rebind_selection_plan(Some("abc"), Some(1)),
            RebindSelectionPlan {
                needs_record_choice: false,
                needs_submission_choice: false,
            }
        );
    }

    #[test]
    fn select_submission_for_record_accepts_matching_submission_id() {
        let submissions = vec![
            SubmissionRecord {
                submission_id: 1,
                submitter: "u".to_string(),
                verdict: "WA".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                submitted_at: None,
            },
            SubmissionRecord {
                submission_id: 2,
                submitter: "u".to_string(),
                verdict: "AC".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                submitted_at: None,
            },
        ];

        let selected = select_submission_for_record("P1001", None, &submissions, Some(2)).unwrap();
        assert_eq!(selected.submission_id, 2);
    }

    #[test]
    fn select_submission_for_record_rejects_missing_submission_id() {
        let submissions = vec![SubmissionRecord {
            submission_id: 1,
            submitter: "u".to_string(),
            verdict: "WA".to_string(),
            score: None,
            time_ms: None,
            memory_mb: None,
            submitted_at: None,
        }];

        let error = select_submission_for_record("P1001", None, &submissions, Some(2)).unwrap_err();
        assert!(format!("{error}").contains("不属于题目 P1001"));
    }

    #[test]
    fn history_records_for_file_only_returns_matching_paths() {
        let records = vec![
            HistoricalSolveRecord {
                revision: "a".to_string(),
                record: SolveRecord {
                    problem_id: "P1001".to_string(),
                    title: "A".to_string(),
                    verdict: "AC".to_string(),
                    difficulty: "入门".to_string(),
                    tags: vec![],
                    submission_id: Some(1),
                    submission_time: None,
                    file_name: "P1001.cpp".to_string(),
                    source_order: 0,
                },
            },
            HistoricalSolveRecord {
                revision: "b".to_string(),
                record: SolveRecord {
                    problem_id: "P1001".to_string(),
                    title: "A".to_string(),
                    verdict: "WA".to_string(),
                    difficulty: "入门".to_string(),
                    tags: vec![],
                    submission_id: Some(2),
                    submission_time: None,
                    file_name: "nested/P1001.cpp".to_string(),
                    source_order: 1,
                },
            },
            HistoricalSolveRecord {
                revision: "c".to_string(),
                record: SolveRecord {
                    problem_id: "P1002".to_string(),
                    title: "B".to_string(),
                    verdict: "AC".to_string(),
                    difficulty: "入门".to_string(),
                    tags: vec![],
                    submission_id: Some(3),
                    submission_time: None,
                    file_name: "nested/P1001.cpp".to_string(),
                    source_order: 2,
                },
            },
        ];

        let filtered = history_records_for_file(&records, "nested/P1001.cpp", "P1001");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].revision, "b");
    }

    #[test]
    fn print_record_list_handles_empty_records() {
        print_record_list(&[]);
    }

    #[tokio::test]
    async fn stats_command_requires_valid_jj_workspace() {
        let workspace =
            std::env::temp_dir().join(format!("aclog-stats-no-jj-{}", std::process::id()));
        std::fs::create_dir_all(&workspace).unwrap();

        let result = run_command(Commands::Stats {
            workspace: workspace.clone(),
        })
        .await;

        assert!(result.is_err());
        let message = format!("{}", result.unwrap_err());
        assert!(message.contains("未找到 jj 工作区"));

        std::fs::remove_dir_all(workspace).unwrap();
    }

    #[tokio::test]
    async fn select_record_for_rebind_rejects_empty_history() {
        let workspace =
            std::env::temp_dir().join(format!("aclog-record-empty-{}", std::process::id()));
        std::fs::create_dir_all(workspace.join(".jj")).unwrap();
        let paths = AclogPaths::new(workspace.clone()).unwrap();
        let target = SolutionFileTarget {
            problem_id: "P1001".to_string(),
            repo_relative_path: "P1001.cpp".to_string(),
        };

        let error = select_record_for_rebind(&paths, &target, &[], None)
            .await
            .unwrap_err();
        assert!(format!("{error}").contains("没有可重绑的记录"));

        std::fs::remove_dir_all(workspace).unwrap();
    }
}
