use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::PathBuf,
};

use chrono::{FixedOffset, Utc};
use color_eyre::{
    Result,
    eyre::{WrapErr, eyre},
};
use futures::stream::{self, StreamExt, TryStreamExt};
use tracing::{debug, info, instrument};

use crate::{
    config::AclogPaths,
    domain::{
        problem::ProblemMetadata,
        record::SyncSelection,
        submission::SubmissionRecord,
        sync_batch::{
            SyncBatchSession, SyncChangeKind, SyncItemStatus, SyncSessionChoice, SyncSessionItem,
            SyncStoredDecision, SyncWarning, SyncWarningCode,
        },
    },
    problem::{extract_problem_target, human_problem_id, is_atcoder_task_id, is_luogu_problem_id},
    ui::interaction::{SyncBatchDetailAction, SyncBatchReviewAction, UserInterface},
};

use super::{
    deps::AppDeps,
    support::{load_record_index, planned_commit},
};

enum PreparedSyncSession {
    Ready(SyncBatchSession),
    Quit,
}

enum ProcessSessionItemOutcome {
    Updated,
    Back,
    Quit,
}

struct FreshSyncSessionBuild {
    session: SyncBatchSession,
    submissions_by_problem: HashMap<String, Vec<SubmissionRecord>>,
    metadata_by_problem: HashMap<String, Option<ProblemMetadata>>,
}

const SYNC_SUBMISSION_PREFETCH_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncOptions {
    pub dry_run: bool,
    pub resume: bool,
    pub rebuild: bool,
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(
    workspace: PathBuf,
    options: SyncOptions,
    deps: &impl AppDeps,
    ui: &impl UserInterface,
) -> Result<()> {
    info!("开始同步");

    let paths = AclogPaths::new(workspace)?;
    let config = crate::config::load_config(&paths)?;
    deps.ensure_workspace().await?;
    let record_index = load_record_index(deps).await?;
    let changed_files = deps.detect_working_copy_changes().await?;
    info!(changed_files = changed_files.len(), "已收集变更文件");
    debug!(?changed_files, "变更文件详情");

    let FreshSyncSessionBuild {
        session: fresh_session,
        mut submissions_by_problem,
        mut metadata_by_problem,
    } = build_fresh_session(&paths, &config, deps, &record_index, &changed_files).await?;
    if options.dry_run {
        deps.write_output(&render_sync_preview(&fresh_session))?;
        info!(rows = fresh_session.items.len(), "dry-run 预览已输出");
        return Ok(());
    }

    let PreparedSyncSession::Ready(mut session) =
        load_or_prepare_session(&paths, options, fresh_session, ui)?
    else {
        info!("已退出 sync 恢复界面，保留现有批次状态");
        return Ok(());
    };
    save_sync_session(&paths, &session)?;

    while session
        .items
        .iter()
        .any(|item| item.status == SyncItemStatus::Pending)
    {
        match ui.review_sync_batch_action(&paths.workspace_root, &session)? {
            SyncBatchReviewAction::Pause => {
                save_sync_session(&paths, &session)?;
                info!("已保留未完成同步批次，后续可恢复");
                return Ok(());
            }
            SyncBatchReviewAction::Open(index) => {
                if index >= session.items.len() {
                    continue;
                }
                if session.items[index].status != SyncItemStatus::Pending {
                    continue;
                }
                match process_session_item(
                    &mut session.items[index],
                    &paths,
                    &config,
                    deps,
                    ui,
                    &record_index,
                    &mut submissions_by_problem,
                    &mut metadata_by_problem,
                )
                .await?
                {
                    ProcessSessionItemOutcome::Updated => {
                        save_sync_session(&paths, &session)?;
                    }
                    ProcessSessionItemOutcome::Back => {}
                    ProcessSessionItemOutcome::Quit => {
                        save_sync_session(&paths, &session)?;
                        info!("已退出 sync 详情页，保留未完成批次以便恢复");
                        return Ok(());
                    }
                }
            }
            SyncBatchReviewAction::Decide { index, selection } => {
                if index >= session.items.len() {
                    continue;
                }
                if session.items[index].status != SyncItemStatus::Pending {
                    continue;
                }
                apply_selection(&mut session.items[index], selection, &record_index)?;
                save_sync_session(&paths, &session)?;
            }
        }
    }

    let commits = build_commits_from_session(
        &session,
        &paths,
        &config,
        deps,
        &mut submissions_by_problem,
        &mut metadata_by_problem,
    )
    .await?;
    if !commits.is_empty() {
        deps.create_commits(&commits).await?;
    }
    delete_sync_session(&paths)?;

    info!(planned_commits = commits.len(), "同步完成");
    Ok(())
}

async fn build_fresh_session(
    paths: &AclogPaths,
    config: &crate::config::AppConfig,
    deps: &impl AppDeps,
    index: &crate::domain::record_index::RecordIndex,
    changed_files: &[crate::vcs::ProblemFileChange],
) -> Result<FreshSyncSessionBuild> {
    let mut items = Vec::with_capacity(changed_files.len());
    for change in changed_files {
        let Some(target) = extract_problem_target(&change.path) else {
            continue;
        };
        let kind = change_kind(change.kind);
        items.push(SyncSessionItem {
            file: change.path.clone(),
            problem_id: Some(target.global_id),
            provider: target.provider,
            contest: None,
            kind,
            status: SyncItemStatus::Pending,
            submissions: None,
            default_submission_id: None,
            decision: None,
            warnings: Vec::new(),
            invalid_reason: None,
        });
    }

    let mut seen_problem_ids = HashSet::new();
    let all_problem_ids = items
        .iter()
        .filter_map(|item| item.problem_id.clone())
        .filter(|problem_id| seen_problem_ids.insert(problem_id.clone()))
        .collect::<Vec<_>>();
    let metadata_by_problem =
        stream::iter(all_problem_ids.iter().cloned().map(|problem_id| async {
            let metadata = deps
                .resolve_problem_metadata(config, paths, &problem_id)
                .await?;
            Ok::<_, color_eyre::Report>((problem_id, metadata))
        }))
        .buffer_unordered(SYNC_SUBMISSION_PREFETCH_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .collect::<HashMap<_, _>>();

    let mut seen_problem_ids = HashSet::new();
    let active_problem_ids = items
        .iter()
        .filter(|item| matches!(item.kind, SyncChangeKind::Active))
        .filter_map(|item| item.problem_id.clone())
        .filter(|problem_id| seen_problem_ids.insert(problem_id.clone()))
        .collect::<Vec<_>>();

    let submissions_by_problem =
        stream::iter(active_problem_ids.into_iter().map(|problem_id| async {
            let submissions = deps
                .fetch_problem_submissions(config, paths, &problem_id)
                .await?;
            Ok::<_, color_eyre::Report>((problem_id, submissions))
        }))
        .buffer_unordered(SYNC_SUBMISSION_PREFETCH_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .collect::<HashMap<_, _>>();

    for item in &mut items {
        if let Some(problem_id) = item.problem_id.as_deref() {
            if let Some(metadata) = metadata_by_problem
                .get(problem_id)
                .and_then(|item| item.as_ref())
            {
                item.provider = metadata.provider;
                item.contest = metadata.contest.clone();
            }
        }
        if !matches!(item.kind, SyncChangeKind::Active) {
            continue;
        }
        let problem_id = item
            .problem_id
            .as_deref()
            .expect("active sync item has problem id");
        let submissions = submissions_by_problem
            .get(problem_id)
            .expect("prefetched submissions should exist for active item");
        item.submissions = Some(submissions.len());
        item.default_submission_id = submissions.first().map(|record| record.submission_id);
        if let Some(duplicate_warning) = duplicate_warning_for_default(index, item) {
            item.warnings.push(duplicate_warning);
        }
    }

    Ok(FreshSyncSessionBuild {
        session: SyncBatchSession {
            created_at: now_in_luogu_timezone(),
            items,
        },
        submissions_by_problem,
        metadata_by_problem,
    })
}

fn load_or_prepare_session(
    paths: &AclogPaths,
    options: SyncOptions,
    fresh_session: SyncBatchSession,
    ui: &impl UserInterface,
) -> Result<PreparedSyncSession> {
    let existing = load_sync_session(paths)?;
    match existing {
        Some(existing) if !options.rebuild => {
            let choice = if options.resume {
                SyncSessionChoice::Resume
            } else {
                ui.choose_sync_session_action(&paths.workspace_root, &existing)?
            };
            match choice {
                SyncSessionChoice::Resume => Ok(PreparedSyncSession::Ready(merge_sync_sessions(
                    fresh_session,
                    existing,
                ))),
                SyncSessionChoice::Rebuild => Ok(PreparedSyncSession::Ready(fresh_session)),
                SyncSessionChoice::Quit => Ok(PreparedSyncSession::Quit),
            }
        }
        Some(_) => Ok(PreparedSyncSession::Ready(fresh_session)),
        None if options.resume => Err(eyre!("当前没有可恢复的 sync 批次")),
        None => Ok(PreparedSyncSession::Ready(fresh_session)),
    }
}

fn merge_sync_sessions(
    mut fresh: SyncBatchSession,
    existing: SyncBatchSession,
) -> SyncBatchSession {
    let mut existing_by_key = existing
        .items
        .into_iter()
        .map(|item| {
            (
                (item.file.clone(), item.problem_id.clone(), item.kind),
                item,
            )
        })
        .collect::<BTreeMap<_, _>>();
    for item in &mut fresh.items {
        let key = (item.file.clone(), item.problem_id.clone(), item.kind);
        if let Some(previous) = existing_by_key.remove(&key) {
            if matches!(
                previous.status,
                SyncItemStatus::Planned | SyncItemStatus::Skipped | SyncItemStatus::Committed
            ) {
                item.status = previous.status;
                item.decision = previous.decision;
                if item.contest.is_none() {
                    item.contest = previous.contest;
                }
            }
        }
    }
    for (_, mut stale) in existing_by_key {
        if stale.problem_id.as_deref().is_none_or(|problem_id| {
            !is_luogu_problem_id(problem_id)
                && !is_atcoder_task_id(problem_id)
                && crate::problem::split_global_problem_id(problem_id).is_none()
        }) {
            continue;
        }
        stale.status = SyncItemStatus::Invalid;
        stale.invalid_reason = Some("当前工作区状态已变化，该批次项已失效".to_string());
        stale.warnings.push(SyncWarning {
            code: SyncWarningCode::InvalidatedByWorkspace,
            message: "当前工作区状态已变化，该批次项已失效".to_string(),
        });
        fresh.items.push(stale);
    }
    fresh
}

async fn process_session_item(
    item: &mut SyncSessionItem,
    paths: &AclogPaths,
    config: &crate::config::AppConfig,
    deps: &impl AppDeps,
    ui: &impl UserInterface,
    index: &crate::domain::record_index::RecordIndex,
    submissions_by_problem: &mut HashMap<String, Vec<SubmissionRecord>>,
    metadata_by_problem: &mut HashMap<String, Option<ProblemMetadata>>,
) -> Result<ProcessSessionItemOutcome> {
    let metadata = match item.problem_id.as_deref() {
        Some(problem_id) => {
            ensure_problem_metadata(metadata_by_problem, problem_id, paths, config, deps).await?
        }
        None => None,
    };
    if item.contest.is_none() {
        item.contest = metadata.as_ref().and_then(|item| item.contest.clone());
    }
    let submissions = if matches!(item.kind, SyncChangeKind::Active) {
        match item.problem_id.as_deref() {
            Some(problem_id) => {
                ensure_problem_submissions(submissions_by_problem, problem_id, paths, config, deps)
                    .await?;
                submissions_by_problem
                    .get(problem_id)
                    .cloned()
                    .unwrap_or_default()
            }
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };

    loop {
        match ui.select_sync_batch_detail_action(item, metadata.as_ref(), &submissions)? {
            SyncBatchDetailAction::Back => return Ok(ProcessSessionItemOutcome::Back),
            SyncBatchDetailAction::Decide(selection) => {
                if apply_selection(item, selection, index)? {
                    return Ok(ProcessSessionItemOutcome::Updated);
                }
            }
            SyncBatchDetailAction::Quit => return Ok(ProcessSessionItemOutcome::Quit),
        }
    }
}

fn apply_selection(
    item: &mut SyncSessionItem,
    selection: SyncSelection,
    index: &crate::domain::record_index::RecordIndex,
) -> Result<bool> {
    match selection {
        SyncSelection::Submission(record) => {
            if let Some(problem_id) = record.problem_id.as_deref() {
                if Some(problem_id) != item.problem_id.as_deref() {
                    item.warnings.push(SyncWarning {
                        code: SyncWarningCode::SubmissionProblemMismatch,
                        message: format!(
                            "选择的 submission 题号为 {problem_id}，与文件题号 {} 不一致",
                            item.problem_id.as_deref().unwrap_or("-")
                        ),
                    });
                    return Ok(false);
                }
            }

            let duplicate = index
                .timeline_for_file(&item.file)
                .first()
                .and_then(|record| record.record.submission_id)
                .is_some_and(|submission_id| submission_id == record.submission_id);
            let already_warned = item
                .warnings
                .iter()
                .any(|warning| warning.code == SyncWarningCode::DuplicateSubmission);
            if duplicate && !already_warned {
                item.warnings.push(SyncWarning {
                    code: SyncWarningCode::DuplicateSubmission,
                    message: format!(
                        "文件 {} 的最新记录已绑定 submission {}；再次选择同一 submission 需要再次确认",
                        item.file, record.submission_id
                    ),
                });
                return Ok(false);
            }

            item.status = SyncItemStatus::Planned;
            item.decision = Some(SyncStoredDecision::Submission {
                submission_id: record.submission_id,
            });
            Ok(true)
        }
        SyncSelection::Chore => {
            item.status = SyncItemStatus::Planned;
            item.decision = Some(SyncStoredDecision::Chore);
            Ok(true)
        }
        SyncSelection::Delete => {
            item.status = SyncItemStatus::Planned;
            item.decision = Some(SyncStoredDecision::Delete);
            Ok(true)
        }
        SyncSelection::Skip => {
            item.status = SyncItemStatus::Skipped;
            item.decision = Some(SyncStoredDecision::Skip);
            Ok(true)
        }
    }
}

async fn build_commits_from_session(
    session: &SyncBatchSession,
    paths: &AclogPaths,
    config: &crate::config::AppConfig,
    deps: &impl AppDeps,
    submissions_by_problem: &mut HashMap<String, Vec<SubmissionRecord>>,
    metadata_by_problem: &mut HashMap<String, Option<ProblemMetadata>>,
) -> Result<Vec<(String, String)>> {
    let mut commits = Vec::new();
    for item in &session.items {
        if item.status != SyncItemStatus::Planned {
            continue;
        }
        let problem_id = item
            .problem_id
            .as_deref()
            .ok_or_else(|| eyre!("批次项 {} 缺少题号，无法生成提交", item.file))?;
        let metadata =
            ensure_problem_metadata(metadata_by_problem, problem_id, paths, config, deps).await?;
        let selection = match item
            .decision
            .as_ref()
            .ok_or_else(|| eyre!("批次项 {} 缺少决策结果", item.file))?
        {
            SyncStoredDecision::Submission { submission_id } => {
                ensure_problem_submissions(submissions_by_problem, problem_id, paths, config, deps)
                    .await?;
                let submissions = submissions_by_problem
                    .get(problem_id)
                    .cloned()
                    .unwrap_or_default();
                let submission = submissions
                    .into_iter()
                    .find(|record| record.submission_id == *submission_id)
                    .ok_or_else(|| {
                        eyre!(
                            "批次项 {} 绑定的 submission {} 已不可用，无法继续提交",
                            item.file,
                            submission_id
                        )
                    })?;
                SyncSelection::Submission(submission)
            }
            SyncStoredDecision::Chore => SyncSelection::Chore,
            SyncStoredDecision::Delete => SyncSelection::Delete,
            SyncStoredDecision::Skip => continue,
        };
        if let Some(commit) = planned_commit(problem_id, &item.file, metadata.as_ref(), &selection)
        {
            commits.push(commit);
        }
    }
    Ok(commits)
}

async fn ensure_problem_metadata(
    metadata_by_problem: &mut HashMap<String, Option<ProblemMetadata>>,
    problem_id: &str,
    paths: &AclogPaths,
    config: &crate::config::AppConfig,
    deps: &impl AppDeps,
) -> Result<Option<ProblemMetadata>> {
    if !metadata_by_problem.contains_key(problem_id) {
        let metadata = deps
            .resolve_problem_metadata(config, paths, problem_id)
            .await?;
        metadata_by_problem.insert(problem_id.to_string(), metadata);
    }
    Ok(metadata_by_problem
        .get(problem_id)
        .cloned()
        .expect("metadata should exist after insertion"))
}

fn duplicate_warning_for_default(
    index: &crate::domain::record_index::RecordIndex,
    item: &SyncSessionItem,
) -> Option<SyncWarning> {
    let default_submission_id = item.default_submission_id?;
    let latest_submission_id = index
        .timeline_for_file(&item.file)
        .first()
        .and_then(|record| record.record.submission_id)?;
    if latest_submission_id != default_submission_id {
        return None;
    }
    Some(SyncWarning {
        code: SyncWarningCode::DuplicateSubmission,
        message: format!(
            "默认候选 submission {} 与文件当前最新记录重复绑定",
            default_submission_id
        ),
    })
}

async fn ensure_problem_submissions(
    submissions_by_problem: &mut HashMap<String, Vec<SubmissionRecord>>,
    problem_id: &str,
    paths: &AclogPaths,
    config: &crate::config::AppConfig,
    deps: &impl AppDeps,
) -> Result<()> {
    if submissions_by_problem.contains_key(problem_id) {
        return Ok(());
    }
    let submissions = deps
        .fetch_problem_submissions(config, paths, problem_id)
        .await?;
    submissions_by_problem.insert(problem_id.to_string(), submissions);
    Ok(())
}

fn load_sync_session(paths: &AclogPaths) -> Result<Option<SyncBatchSession>> {
    if !paths.sync_session_file.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&paths.sync_session_file).wrap_err_with(|| {
        format!(
            "读取 sync 会话文件 {} 失败",
            paths.sync_session_file.display()
        )
    })?;
    let session: SyncBatchSession = toml::from_str(&raw).wrap_err("sync 会话文件格式无效")?;
    Ok(Some(session))
}

fn save_sync_session(paths: &AclogPaths, session: &SyncBatchSession) -> Result<()> {
    if let Some(parent) = paths.sync_session_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &paths.sync_session_file,
        format!("{}\n", toml::to_string_pretty(session)?),
    )
    .wrap_err_with(|| {
        format!(
            "写入 sync 会话文件 {} 失败",
            paths.sync_session_file.display()
        )
    })
}

fn delete_sync_session(paths: &AclogPaths) -> Result<()> {
    if paths.sync_session_file.exists() {
        fs::remove_file(&paths.sync_session_file).wrap_err_with(|| {
            format!(
                "删除 sync 会话文件 {} 失败",
                paths.sync_session_file.display()
            )
        })?;
    }
    Ok(())
}

fn render_sync_preview(session: &SyncBatchSession) -> String {
    if session.items.is_empty() {
        return "当前没有待处理的题目文件变更\n".to_string();
    }

    let mut lines = vec![crate::output_style::header(
        "文件\t题号\t来源\t变更类型\t当前状态\t提交记录数\t默认候选\t告警",
    )];
    for row in &session.items {
        let submissions = row
            .submissions
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        let default_submission = row
            .default_submission_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        let warnings = if let Some(reason) = row.invalid_reason.as_deref() {
            reason.to_string()
        } else if row.warnings.is_empty() {
            "-".to_string()
        } else {
            row.warnings
                .iter()
                .map(|warning| warning.message.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        };
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.file,
            row.problem_id
                .as_deref()
                .map(human_problem_id)
                .unwrap_or_else(|| "-".to_string()),
            crate::problem::provider_label(row.provider),
            crate::output_style::sync_change_kind(row.kind),
            crate::output_style::sync_status(row),
            submissions,
            default_submission,
            if warnings == "-" {
                crate::output_style::muted(&warnings)
            } else {
                crate::output_style::warning(&warnings)
            }
        ));
    }
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
fn preview_status(item: &SyncSessionItem) -> &'static str {
    match item.status {
        SyncItemStatus::Pending => match item.kind {
            SyncChangeKind::Deleted => "等待确认删除",
            SyncChangeKind::Active if item.submissions.unwrap_or(0) == 0 => "未找到提交记录",
            SyncChangeKind::Active => "等待选择提交记录",
        },
        SyncItemStatus::Planned => "已决待提交",
        SyncItemStatus::Skipped => "已跳过",
        SyncItemStatus::Committed => "已提交",
        SyncItemStatus::Invalid => "已失效",
    }
}

fn change_kind(kind: crate::vcs::ProblemFileChangeKind) -> SyncChangeKind {
    match kind {
        crate::vcs::ProblemFileChangeKind::Active => SyncChangeKind::Active,
        crate::vcs::ProblemFileChangeKind::Deleted => SyncChangeKind::Deleted,
    }
}

fn now_in_luogu_timezone() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).expect("固定时区偏移应当有效"))
}

#[cfg(test)]
mod tests {
    use super::{
        SyncOptions, SyncStoredDecision, merge_sync_sessions, preview_status, render_sync_preview,
    };
    use crate::domain::sync_batch::{
        SyncBatchSession, SyncChangeKind, SyncItemStatus, SyncSessionItem, SyncWarning,
        SyncWarningCode,
    };

    #[test]
    fn sync_options_default_to_non_recovery_mode() {
        assert_eq!(SyncOptions::default(), SyncOptions::default());
    }

    #[test]
    fn render_sync_preview_outputs_status_and_warning_columns() {
        let text = render_sync_preview(&SyncBatchSession {
            created_at: super::now_in_luogu_timezone(),
            items: vec![
                SyncSessionItem {
                    file: "P1001.cpp".to_string(),
                    problem_id: Some("luogu:P1001".to_string()),
                    provider: crate::problem::ProblemProvider::Luogu,
                    contest: None,
                    kind: SyncChangeKind::Active,
                    status: SyncItemStatus::Pending,
                    submissions: Some(2),
                    default_submission_id: Some(42),
                    decision: None,
                    warnings: vec![SyncWarning {
                        code: SyncWarningCode::DuplicateSubmission,
                        message: "默认候选重复".to_string(),
                    }],
                    invalid_reason: None,
                },
                SyncSessionItem {
                    file: "notes.txt".to_string(),
                    problem_id: None,
                    provider: crate::problem::ProblemProvider::Unknown,
                    contest: None,
                    kind: SyncChangeKind::Active,
                    status: SyncItemStatus::Invalid,
                    submissions: None,
                    default_submission_id: None,
                    decision: None,
                    warnings: Vec::new(),
                    invalid_reason: Some("无法识别题号".to_string()),
                },
            ],
        });

        assert!(text.contains("告警"));
        assert!(text.contains("默认候选重复"));
        assert!(text.contains("无法识别题号"));
    }

    #[test]
    fn merge_sync_sessions_preserves_decisions_and_marks_stale_items_invalid() {
        let fresh = SyncBatchSession {
            created_at: super::now_in_luogu_timezone(),
            items: vec![SyncSessionItem {
                file: "P1001.cpp".to_string(),
                problem_id: Some("luogu:P1001".to_string()),
                provider: crate::problem::ProblemProvider::Luogu,
                contest: None,
                kind: SyncChangeKind::Active,
                status: SyncItemStatus::Pending,
                submissions: Some(1),
                default_submission_id: Some(1),
                decision: None,
                warnings: Vec::new(),
                invalid_reason: None,
            }],
        };
        let existing = SyncBatchSession {
            created_at: super::now_in_luogu_timezone(),
            items: vec![
                SyncSessionItem {
                    file: "P1001.cpp".to_string(),
                    problem_id: Some("luogu:P1001".to_string()),
                    provider: crate::problem::ProblemProvider::Luogu,
                    contest: None,
                    kind: SyncChangeKind::Active,
                    status: SyncItemStatus::Planned,
                    submissions: Some(1),
                    default_submission_id: Some(1),
                    decision: Some(SyncStoredDecision::Submission { submission_id: 1 }),
                    warnings: Vec::new(),
                    invalid_reason: None,
                },
                SyncSessionItem {
                    file: "gone/P1002.cpp".to_string(),
                    problem_id: Some("luogu:P1002".to_string()),
                    provider: crate::problem::ProblemProvider::Luogu,
                    contest: None,
                    kind: SyncChangeKind::Active,
                    status: SyncItemStatus::Skipped,
                    submissions: Some(0),
                    default_submission_id: None,
                    decision: Some(SyncStoredDecision::Skip),
                    warnings: Vec::new(),
                    invalid_reason: None,
                },
            ],
        };

        let merged = merge_sync_sessions(fresh, existing);
        assert_eq!(merged.items[0].status, SyncItemStatus::Planned);
        assert_eq!(merged.items.len(), 2);
        assert_eq!(merged.items[1].status, SyncItemStatus::Invalid);
    }

    #[test]
    fn preview_status_reflects_pending_variants() {
        assert_eq!(
            preview_status(&SyncSessionItem {
                file: "P1001.cpp".to_string(),
                problem_id: Some("luogu:P1001".to_string()),
                provider: crate::problem::ProblemProvider::Luogu,
                contest: None,
                kind: SyncChangeKind::Deleted,
                status: SyncItemStatus::Pending,
                submissions: None,
                default_submission_id: None,
                decision: None,
                warnings: Vec::new(),
                invalid_reason: None,
            }),
            "等待确认删除"
        );
    }
}
