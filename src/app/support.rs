use std::path::Path;

use color_eyre::Result;
use color_eyre::eyre::{WrapErr, eyre};

use crate::{
    config::AclogPaths,
    domain::{
        problem::ProblemMetadata,
        record::FileRecordSummary,
        record::{HistoricalSolveRecord, SyncSelection},
        submission::SubmissionRecord,
    },
    ui::interaction::UserInterface,
};

use super::deps::RepoGateway;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SolutionFileTarget {
    pub(crate) problem_id: String,
    pub(crate) repo_relative_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RebindSelectionPlan {
    pub(crate) needs_record_choice: bool,
    pub(crate) needs_submission_choice: bool,
}

pub(crate) async fn resolve_solution_file_target(
    paths: &AclogPaths,
    file: &Path,
    repo: &impl RepoGateway,
) -> Result<SolutionFileTarget> {
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
    if !repo
        .is_tracked_file(&paths.workspace_root, &repo_relative_path)
        .await?
    {
        return Err(eyre!("文件 {} 未被当前 jj 工作区跟踪", repo_relative_path));
    }
    Ok(SolutionFileTarget {
        problem_id,
        repo_relative_path,
    })
}

pub(crate) fn submission_selection_plan(submission_id: Option<u64>) -> bool {
    submission_id.is_none()
}

pub(crate) fn rebind_selection_plan(
    record_rev: Option<&str>,
    submission_id: Option<u64>,
) -> RebindSelectionPlan {
    RebindSelectionPlan {
        needs_record_choice: record_rev.is_none(),
        needs_submission_choice: submission_id.is_none(),
    }
}

pub(crate) fn history_records_for_file(
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

pub(crate) fn select_submission_for_record(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
    submission_id: Option<u64>,
    ui: &impl UserInterface,
) -> Result<SubmissionRecord> {
    if let Some(submission_id) = submission_id {
        return submissions
            .iter()
            .find(|record| record.submission_id == submission_id)
            .cloned()
            .ok_or_else(|| eyre!("提交记录 {} 不属于题目 {}", submission_id, problem_id));
    }

    ui.select_record_submission(problem_id, metadata, submissions)?
        .ok_or_else(|| eyre!("已取消选择提交记录"))
}

pub(crate) async fn select_record_for_rebind(
    paths: &AclogPaths,
    target: &SolutionFileTarget,
    candidates: &[HistoricalSolveRecord],
    record_rev: Option<&str>,
    repo: &impl RepoGateway,
    ui: &impl UserInterface,
) -> Result<HistoricalSolveRecord> {
    if candidates.is_empty() {
        return Err(eyre!(
            "文件 {} 当前没有可重绑的记录",
            target.repo_relative_path
        ));
    }

    if let Some(record_rev) = record_rev {
        let revision = repo
            .resolve_single_commit_id(&paths.workspace_root, record_rev)
            .await?;
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

    ui.select_record_to_rebind(&target.problem_id, &target.repo_relative_path, candidates)?
        .ok_or_else(|| eyre!("已取消选择要重写的记录"))
}

pub(crate) fn planned_commit(
    problem_id: &str,
    file: &str,
    metadata: Option<&ProblemMetadata>,
    selection: &SyncSelection,
) -> Option<(String, String)> {
    match selection {
        SyncSelection::Skip => None,
        _ => Some((
            file.to_string(),
            crate::commit_format::build_commit_message(problem_id, file, metadata, selection),
        )),
    }
}

pub(crate) fn selection_kind(selection: &SyncSelection) -> &'static str {
    match selection {
        SyncSelection::Submission(_) => "submission",
        SyncSelection::Chore => "chore",
        SyncSelection::Delete => "delete",
        SyncSelection::Skip => "skip",
    }
}

pub(crate) fn should_fetch_submissions(change_kind: crate::vcs::ProblemFileChangeKind) -> bool {
    matches!(change_kind, crate::vcs::ProblemFileChangeKind::Active)
}

pub(crate) fn render_record_list(records: &[FileRecordSummary]) -> String {
    if records.is_empty() {
        return "当前工作区还没有已记录的解法文件\n".to_string();
    }

    let mut lines = vec!["FILE\tPID\tVERDICT\tDIFF\tSUBMISSION\tRECORDED-AT\tTITLE".to_string()];
    for record in records {
        let submission_id = record
            .submission_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        let recorded_at = record
            .submission_time
            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            record.file_name,
            record.problem_id,
            record.verdict,
            record.difficulty,
            submission_id,
            recorded_at,
            record.title,
        ));
    }
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use crate::domain::record::{HistoricalSolveRecord, SolveRecord};

    use super::history_records_for_file;

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
        ];

        let filtered = history_records_for_file(&records, "nested/P1001.cpp", "P1001");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].revision, "b");
    }
}
