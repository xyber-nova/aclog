use std::path::Path;

use color_eyre::Result;
use color_eyre::eyre::{WrapErr, eyre};

use crate::{
    config::AclogPaths,
    domain::{
        problem::ProblemMetadata,
        record::FileRecordSummary,
        record::{HistoricalSolveRecord, SolveRecord, SyncSelection},
        record_index::RecordIndex,
        submission::SubmissionRecord,
        training_fields::normalize_optional_training_text,
    },
    problem::human_problem_id,
    ui::interaction::UserInterface,
    utils::{normalize_verdict, verdict_equals},
};

use super::deps::JjRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SolutionFileTarget {
    pub(crate) problem_id: String,
    pub(crate) provider: crate::problem::ProblemProvider,
    pub(crate) raw_problem_id: String,
    pub(crate) repo_relative_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RebindSelectionPlan {
    pub(crate) needs_record_choice: bool,
    pub(crate) needs_submission_choice: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordListQuery {
    pub problem_id: Option<String>,
    pub file_name: Option<String>,
    pub verdict: Option<String>,
    pub difficulty: Option<String>,
    pub tag: Option<String>,
    pub json: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrainingFieldsPatch {
    pub note: Option<String>,
    pub mistakes: Option<String>,
    pub insight: Option<String>,
    pub confidence: Option<String>,
    pub source_kind: Option<String>,
    pub time_spent: Option<String>,
}

pub(crate) async fn resolve_solution_file_target(
    paths: &AclogPaths,
    file: &Path,
    repo: &impl JjRepository,
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
    let parsed = crate::problem::extract_problem_target(file_name).ok_or_else(|| {
        eyre!(
            "无法从文件名 {} 提取受支持的题目标识（当前支持 Luogu / AtCoder）",
            file_name
        )
    })?;
    if !repo.is_tracked_file(&repo_relative_path).await? {
        return Err(eyre!("文件 {} 未被当前 jj 工作区跟踪", repo_relative_path));
    }
    Ok(SolutionFileTarget {
        problem_id: parsed.global_id,
        provider: parsed.provider,
        raw_problem_id: parsed.raw_id,
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

pub(crate) async fn load_record_index(repo: &impl JjRepository) -> Result<RecordIndex> {
    repo.load_record_index().await
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
            .ok_or_else(|| {
                eyre!(
                    "提交记录 {} 不属于题目 {}",
                    submission_id,
                    human_problem_id(problem_id)
                )
            });
    }

    ui.select_record_submission(problem_id, metadata, submissions)?
        .ok_or_else(|| eyre!("已取消选择提交记录"))
}

pub(crate) async fn select_record_for_rebind(
    target: &SolutionFileTarget,
    candidates: &[HistoricalSolveRecord],
    record_rev: Option<&str>,
    repo: &impl JjRepository,
    ui: &impl UserInterface,
) -> Result<HistoricalSolveRecord> {
    if candidates.is_empty() {
        return Err(eyre!(
            "文件 {} 当前没有可重绑的记录",
            target.repo_relative_path
        ));
    }

    if let Some(record_rev) = record_rev {
        let revision = repo.resolve_revision(record_rev).await?;
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

pub(crate) async fn resolve_record_for_file(
    target: &SolutionFileTarget,
    record_rev: Option<&str>,
    repo: &impl JjRepository,
) -> Result<HistoricalSolveRecord> {
    let index = load_record_index(repo).await?;
    if let Some(record_rev) = record_rev {
        let revision = repo.resolve_revision(record_rev).await?;
        let record = index
            .all_records()
            .iter()
            .find(|entry| entry.revision == revision)
            .cloned()
            .ok_or_else(|| eyre!("`--record-rev` 指定的提交不存在"))?;
        if record.record.file_name != target.repo_relative_path {
            return Err(eyre!(
                "`--record-rev` 指定的提交不是该文件 {} 的标准 solve 记录",
                target.repo_relative_path
            ));
        }
        if record.record.problem_id != target.problem_id {
            return Err(eyre!(
                "`--record-rev` 指定的提交题号与文件 {} 不匹配",
                target.repo_relative_path
            ));
        }
        return Ok(record);
    }

    index
        .timeline_for_file(&target.repo_relative_path)
        .first()
        .cloned()
        .ok_or_else(|| {
            eyre!(
                "文件 {} 当前没有已记录的 solve 记录",
                target.repo_relative_path
            )
        })
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

pub(crate) fn filter_record_summaries(
    records: &[FileRecordSummary],
    query: &RecordListQuery,
) -> Vec<FileRecordSummary> {
    records
        .iter()
        .filter(|record| {
            query
                .problem_id
                .as_deref()
                .is_none_or(|pid| record.problem_id.eq_ignore_ascii_case(pid))
                && query
                    .file_name
                    .as_deref()
                    .is_none_or(|file_name| record.file_name.contains(file_name))
                && query
                    .verdict
                    .as_deref()
                    .is_none_or(|verdict| verdict_equals(&record.verdict, verdict))
                && query
                    .difficulty
                    .as_deref()
                    .is_none_or(|difficulty| record.difficulty == difficulty)
                && query
                    .tag
                    .as_deref()
                    .is_none_or(|tag| record.tags.iter().any(|item| item == tag))
        })
        .cloned()
        .collect()
}

pub(crate) fn render_record_list(records: &[FileRecordSummary]) -> String {
    if records.is_empty() {
        return "当前工作区还没有已记录的解法文件\n".to_string();
    }

    let mut lines = vec![crate::output_style::header(
        "文件\t题号\t结果\t难度\t提交编号\t记录时间\t标题",
    )];
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
            crate::output_style::verdict(&record.verdict),
            record.difficulty,
            submission_id,
            recorded_at,
            record.title,
        ));
    }
    format!("{}\n", lines.join("\n"))
}

pub(crate) fn render_record_list_json(records: &[FileRecordSummary]) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(records)?))
}

fn render_record_detail_lines(record: &HistoricalSolveRecord) -> [String; 19] {
    let solve = &record.record;
    let submission_id = solve
        .submission_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let recorded_at = solve
        .submission_time
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| "-".to_string());
    let tags = if solve.tags.is_empty() {
        "-".to_string()
    } else {
        solve.tags.join(", ")
    };
    let score = solve
        .score
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let time_ms = solve
        .time_ms
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "-".to_string());
    let memory_mb = solve
        .memory_mb
        .map(|value| format!("{value:.1}MB"))
        .unwrap_or_else(|| "-".to_string());

    [
        format!("版本: {}", record.revision),
        format!("题号: {}", solve.problem_id),
        format!("标题: {}", solve.title),
        format!("文件: {}", solve.file_name),
        format!("结果: {}", normalize_verdict(&solve.verdict)),
        format!("分数: {score}"),
        format!("耗时: {time_ms}"),
        format!("内存: {memory_mb}"),
        format!("难度: {}", solve.difficulty),
        format!("来源: {}", solve.source),
        format!("提交编号: {submission_id}"),
        format!("提交时间: {recorded_at}"),
        format!("标签: {tags}"),
        format!("笔记: {}", solve.training.note.as_deref().unwrap_or("-")),
        format!(
            "卡点: {}",
            solve.training.mistakes.as_deref().unwrap_or("-")
        ),
        format!("收获: {}", solve.training.insight.as_deref().unwrap_or("-")),
        format!(
            "熟练度: {}",
            solve.training.confidence.as_deref().unwrap_or("-")
        ),
        format!(
            "完成方式: {}",
            solve.training.source_kind.as_deref().unwrap_or("-")
        ),
        format!(
            "训练耗时: {}",
            solve.training.time_spent.as_deref().unwrap_or("-")
        ),
    ]
}

pub(crate) fn render_record_detail_colored(record: &HistoricalSolveRecord) -> String {
    render_record_detail_lines(record)
        .into_iter()
        .map(|line| {
            let Some((label, value)) = line.split_once(' ') else {
                return line;
            };
            let rendered_value = if label == "结果:" {
                crate::output_style::verdict(value)
            } else {
                value.to_string()
            };
            format!("{} {}", crate::output_style::label(label), rendered_value)
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

pub(crate) fn render_record_detail_json(record: &HistoricalSolveRecord) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(record)?))
}

pub(crate) fn apply_training_patch(
    record: &HistoricalSolveRecord,
    patch: &TrainingFieldsPatch,
) -> Result<SolveRecord> {
    if patch == &TrainingFieldsPatch::default() {
        return Err(eyre!("至少需要提供一个训练字段参数"));
    }

    let mut updated = record.record.clone();
    update_training_field(&mut updated.training.note, patch.note.as_deref());
    update_training_field(&mut updated.training.mistakes, patch.mistakes.as_deref());
    update_training_field(&mut updated.training.insight, patch.insight.as_deref());
    update_training_field(
        &mut updated.training.confidence,
        patch.confidence.as_deref(),
    );
    update_training_field(
        &mut updated.training.source_kind,
        patch.source_kind.as_deref(),
    );
    update_training_field(
        &mut updated.training.time_spent,
        patch.time_spent.as_deref(),
    );
    Ok(updated)
}

fn update_training_field(slot: &mut Option<String>, input: Option<&str>) {
    if let Some(input) = input {
        *slot = normalize_optional_training_text(Some(input));
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::record::{HistoricalSolveRecord, SolveRecord, TrainingFields};

    use super::history_records_for_file;

    #[test]
    fn history_records_for_file_only_returns_matching_paths() {
        let records = vec![
            HistoricalSolveRecord {
                revision: "a".to_string(),
                record: SolveRecord {
                    problem_id: "P1001".to_string(),
                    provider: crate::problem::ProblemProvider::Luogu,
                    title: "A".to_string(),
                    verdict: "AC".to_string(),
                    score: None,
                    time_ms: None,
                    memory_mb: None,
                    difficulty: "入门".to_string(),
                    tags: vec![],
                    source: "Luogu".to_string(),
                    contest: None,
                    submission_id: Some(1),
                    submission_time: None,
                    file_name: "P1001.cpp".to_string(),
                    training: TrainingFields::default(),
                    source_order: 0,
                },
            },
            HistoricalSolveRecord {
                revision: "b".to_string(),
                record: SolveRecord {
                    problem_id: "P1001".to_string(),
                    provider: crate::problem::ProblemProvider::Luogu,
                    title: "A".to_string(),
                    verdict: "WA".to_string(),
                    score: None,
                    time_ms: None,
                    memory_mb: None,
                    difficulty: "入门".to_string(),
                    tags: vec![],
                    source: "Luogu".to_string(),
                    contest: None,
                    submission_id: Some(2),
                    submission_time: None,
                    file_name: "nested/P1001.cpp".to_string(),
                    training: TrainingFields::default(),
                    source_order: 1,
                },
            },
        ];

        let filtered = history_records_for_file(&records, "nested/P1001.cpp", "P1001");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].revision, "b");
    }
}
