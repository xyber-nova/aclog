use std::collections::HashSet;

use chrono::{DateTime, Duration, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    record::{HistoricalSolveRecord, ProblemRecordSummary},
    record_index::RecordIndex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BrowserRootView {
    #[default]
    Files,
    Problems,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BrowserQuery {
    pub root_view: BrowserRootView,
    pub problem_id: Option<String>,
    pub file_name: Option<String>,
    pub verdict: Option<String>,
    pub difficulty: Option<String>,
    pub tag: Option<String>,
    pub days: Option<i64>,
    pub timeline_file: Option<String>,
    pub timeline_problem: Option<String>,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserFileRow {
    pub revision: String,
    pub problem_id: String,
    pub title: String,
    pub file_name: String,
    pub verdict: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
    pub training_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserProblemRow {
    pub problem_id: String,
    pub title: String,
    pub verdict: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub files: Vec<String>,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
    pub latest_revision: String,
    pub training_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTimelineRow {
    pub revision: String,
    pub problem_id: String,
    pub title: String,
    pub file_name: String,
    pub verdict: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
    pub training_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BrowserState {
    pub files: Vec<BrowserFileRow>,
    pub problems: Vec<BrowserProblemRow>,
}

pub fn build_browser_state(index: &RecordIndex) -> BrowserState {
    let files = index
        .current_by_file()
        .iter()
        .map(|item| BrowserFileRow {
            revision: item.revision.clone(),
            problem_id: item.problem_id.clone(),
            title: item.title.clone(),
            file_name: item.file_name.clone(),
            verdict: item.verdict.clone(),
            difficulty: item.difficulty.clone(),
            tags: item.tags.clone(),
            submission_id: item.submission_id,
            submission_time: item.submission_time,
            training_summary: summarize_training_fields(&item.training),
        })
        .collect();
    let problems = index
        .current_by_problem()
        .iter()
        .map(|item| BrowserProblemRow {
            problem_id: item.problem_id.clone(),
            title: item.title.clone(),
            verdict: item.verdict.clone(),
            difficulty: item.difficulty.clone(),
            tags: item.tags.clone(),
            files: item.files.clone(),
            submission_id: item.submission_id,
            submission_time: item.submission_time,
            latest_revision: item.latest_revision.clone(),
            training_summary: summarize_problem_training(index, item),
        })
        .collect();
    BrowserState { files, problems }
}

pub fn filter_browser_files(rows: &[BrowserFileRow], query: &BrowserQuery) -> Vec<BrowserFileRow> {
    rows.iter()
        .filter(|row| {
            matches_browser_row(
                query,
                &row.problem_id,
                &row.file_name,
                &row.verdict,
                &row.difficulty,
                &row.tags,
                row.submission_time,
            )
        })
        .cloned()
        .collect()
}

pub fn filter_browser_problems(
    rows: &[BrowserProblemRow],
    query: &BrowserQuery,
) -> Vec<BrowserProblemRow> {
    rows.iter()
        .filter(|row| {
            let matched_file = query
                .file_name
                .as_deref()
                .is_none_or(|needle| row.files.iter().any(|file| file.contains(needle)));
            matched_file
                && matches_browser_row(
                    query,
                    &row.problem_id,
                    &row.problem_id,
                    &row.verdict,
                    &row.difficulty,
                    &row.tags,
                    row.submission_time,
                )
        })
        .cloned()
        .collect()
}

pub fn filter_timeline_rows(
    rows: &[BrowserTimelineRow],
    query: &BrowserQuery,
) -> Vec<BrowserTimelineRow> {
    rows.iter()
        .filter(|row| {
            matches_browser_row(
                query,
                &row.problem_id,
                &row.file_name,
                &row.verdict,
                &row.difficulty,
                &row.tags,
                row.submission_time,
            )
        })
        .cloned()
        .collect()
}

pub fn timeline_rows_for_file(index: &RecordIndex, file_name: &str) -> Vec<BrowserTimelineRow> {
    index
        .timeline_for_file(file_name)
        .iter()
        .map(to_timeline_row)
        .collect()
}

pub fn timeline_rows_for_problem(index: &RecordIndex, problem_id: &str) -> Vec<BrowserTimelineRow> {
    index
        .timeline_for_problem(problem_id)
        .iter()
        .map(to_timeline_row)
        .collect()
}

pub fn filter_tag_names(
    rows: &[BrowserTimelineRow],
    algorithm_tag_names: Option<&HashSet<String>>,
) -> Vec<String> {
    let mut tags = rows
        .iter()
        .flat_map(|row| row.tags.iter().cloned())
        .filter(|tag| {
            algorithm_tag_names
                .map(|names| names.contains(tag))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    tags
}

fn to_timeline_row(record: &HistoricalSolveRecord) -> BrowserTimelineRow {
    BrowserTimelineRow {
        revision: record.revision.clone(),
        problem_id: record.record.problem_id.clone(),
        title: record.record.title.clone(),
        file_name: record.record.file_name.clone(),
        verdict: record.record.verdict.clone(),
        difficulty: record.record.difficulty.clone(),
        tags: record.record.tags.clone(),
        submission_id: record.record.submission_id,
        submission_time: record.record.submission_time,
        training_summary: summarize_training_fields(&record.record.training),
    }
}

fn summarize_problem_training(index: &RecordIndex, item: &ProblemRecordSummary) -> String {
    index
        .timeline_for_problem(&item.problem_id)
        .first()
        .map(|record| summarize_training_fields(&record.record.training))
        .unwrap_or_else(|| "-".to_string())
}

fn summarize_training_fields(training: &crate::domain::record::TrainingFields) -> String {
    [
        training.note.as_deref(),
        training.mistakes.as_deref(),
        training.insight.as_deref(),
        training.confidence.as_deref(),
    ]
    .into_iter()
    .flatten()
    .find(|value| !value.trim().is_empty())
    .map(ToString::to_string)
    .unwrap_or_else(|| "-".to_string())
}

fn matches_browser_row(
    query: &BrowserQuery,
    problem_id: &str,
    file_name: &str,
    verdict: &str,
    difficulty: &str,
    tags: &[String],
    submission_time: Option<DateTime<FixedOffset>>,
) -> bool {
    query
        .problem_id
        .as_deref()
        .is_none_or(|needle| problem_id.eq_ignore_ascii_case(needle))
        && query
            .file_name
            .as_deref()
            .is_none_or(|needle| file_name.contains(needle))
        && query
            .verdict
            .as_deref()
            .is_none_or(|needle| verdict.eq_ignore_ascii_case(needle))
        && query
            .difficulty
            .as_deref()
            .is_none_or(|needle| difficulty == needle)
        && query
            .tag
            .as_deref()
            .is_none_or(|needle| tags.iter().any(|tag| tag == needle))
        && query
            .days
            .is_none_or(|days| within_days(submission_time, days))
}

fn within_days(submission_time: Option<DateTime<FixedOffset>>, days: i64) -> bool {
    let Some(submission_time) = submission_time else {
        return false;
    };
    submission_time.with_timezone(&Utc) >= Utc::now() - Duration::days(days)
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};

    use super::{
        BrowserQuery, BrowserRootView, build_browser_state, filter_browser_files,
        timeline_rows_for_problem,
    };
    use crate::domain::{
        record::{HistoricalSolveRecord, SolveRecord, TrainingFields},
        record_index::RecordIndex,
    };

    #[test]
    fn browser_state_builds_current_views_and_timeline() {
        let record = HistoricalSolveRecord {
            revision: "rev".to_string(),
            record: SolveRecord {
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                verdict: "AC".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string()],
                source: "Luogu".to_string(),
                submission_id: Some(1),
                submission_time: Some(
                    FixedOffset::east_opt(8 * 3600)
                        .unwrap()
                        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                        .single()
                        .unwrap(),
                ),
                file_name: "P1001.cpp".to_string(),
                training: TrainingFields {
                    note: Some("复习".to_string()),
                    ..TrainingFields::default()
                },
                source_order: 0,
            },
        };
        let index = RecordIndex::build(&[record]);
        let state = build_browser_state(&index);
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.problems.len(), 1);
        assert_eq!(state.files[0].training_summary, "复习");
        assert_eq!(timeline_rows_for_problem(&index, "P1001").len(), 1);
    }

    #[test]
    fn browser_file_filter_applies_intersection() {
        let rows = vec![
            super::BrowserFileRow {
                revision: "rev".to_string(),
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                file_name: "sol/P1001.cpp".to_string(),
                verdict: "AC".to_string(),
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string()],
                submission_id: Some(1),
                submission_time: None,
                training_summary: "-".to_string(),
            },
            super::BrowserFileRow {
                revision: "rev2".to_string(),
                problem_id: "P1002".to_string(),
                title: "B".to_string(),
                file_name: "sol/P1002.cpp".to_string(),
                verdict: "WA".to_string(),
                difficulty: "普及-".to_string(),
                tags: vec!["二分".to_string()],
                submission_id: Some(2),
                submission_time: None,
                training_summary: "-".to_string(),
            },
        ];
        let filtered = filter_browser_files(
            &rows,
            &BrowserQuery {
                root_view: BrowserRootView::Files,
                verdict: Some("AC".to_string()),
                tag: Some("模拟".to_string()),
                ..BrowserQuery::default()
            },
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].problem_id, "P1001");
    }
}
