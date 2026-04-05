use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, FixedOffset};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemMetadata {
    pub id: String,
    pub title: String,
    pub difficulty: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub url: String,
    pub fetched_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionRecord {
    pub submission_id: u64,
    pub submitter: String,
    pub verdict: String,
    pub score: Option<i64>,
    pub time_ms: Option<u64>,
    pub memory_mb: Option<f64>,
    pub submitted_at: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Clone)]
pub enum SyncSelection {
    Submission(SubmissionRecord),
    Chore,
    Delete,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveRecord {
    pub problem_id: String,
    pub title: String,
    pub verdict: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
    pub file_name: String,
    pub source_order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalSolveRecord {
    pub revision: String,
    pub record: SolveRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecordSummary {
    pub revision: String,
    pub problem_id: String,
    pub title: String,
    pub file_name: String,
    pub verdict: String,
    pub difficulty: String,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsSummary {
    pub total_solve_records: usize,
    pub unique_problem_count: usize,
    pub unique_ac_count: usize,
    pub unique_non_ac_count: usize,
    pub verdict_counts: Vec<(String, usize)>,
    pub difficulty_counts: Vec<(String, usize)>,
    pub tag_counts: Vec<(String, usize)>,
}

pub fn build_commit_message(
    problem_id: &str,
    file_name: &str,
    metadata: Option<&ProblemMetadata>,
    selection: &SyncSelection,
) -> String {
    match selection {
        SyncSelection::Submission(record) => build_solve_commit_message(
            problem_id,
            file_name,
            metadata,
            record,
        ),
        SyncSelection::Chore => format!("chore({problem_id}): 本地修改\n\nFile: {file_name}"),
        SyncSelection::Delete => build_delete_commit_message(problem_id, file_name, metadata),
        SyncSelection::Skip => String::new(),
    }
}

pub fn build_solve_commit_message(
    problem_id: &str,
    file_name: &str,
    metadata: Option<&ProblemMetadata>,
    record: &SubmissionRecord,
) -> String {
    let title = metadata
        .map(|item| item.title.as_str())
        .filter(|title| !title.is_empty())
        .unwrap_or("Unknown Problem");
    let tags = metadata
        .map(|item| item.tags.join(", "))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string());
    let difficulty = metadata
        .and_then(|item| item.difficulty.as_deref())
        .unwrap_or("-");
    let source = metadata
        .and_then(|item| item.source.as_deref())
        .unwrap_or("Luogu");
    let submitted_at = record
        .submitted_at
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| "-".to_string());

    format!(
        "solve({problem_id}): {title}\n\nVerdict: {}\nScore: {}\nTime: {}\nMemory: {}\nSubmission-ID: {}\nSubmission-Time: {}\nTags: {}\nDifficulty: {}\nSource: {}\nFile: {}",
        record.verdict,
        record
            .score
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        record
            .time_ms
            .map(|value| format!("{value}ms"))
            .unwrap_or_else(|| "-".to_string()),
        record
            .memory_mb
            .map(|value| format!("{value:.1}MB"))
            .unwrap_or_else(|| "-".to_string()),
        record.submission_id,
        submitted_at,
        tags,
        difficulty,
        source,
        file_name,
    )
}

fn build_delete_commit_message(
    problem_id: &str,
    file_name: &str,
    metadata: Option<&ProblemMetadata>,
) -> String {
    let mut body = Vec::new();
    if let Some(title) = metadata
        .map(|item| item.title.trim())
        .filter(|title| !title.is_empty())
    {
        body.push(format!("Title: {title}"));
    }
    body.push(format!("File: {file_name}"));

    format!("remove({problem_id}): 删除题解文件\n\n{}", body.join("\n"))
}

pub fn parse_solve_records(messages: &[String]) -> Vec<SolveRecord> {
    messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| parse_solve_commit_message(message, index))
        .collect()
}

pub fn parse_historical_solve_records(
    entries: &[(String, String)],
) -> Vec<HistoricalSolveRecord> {
    entries
        .iter()
        .enumerate()
        .filter_map(|(index, (revision, message))| {
            parse_solve_commit_message(message, index).map(|record| HistoricalSolveRecord {
                revision: revision.clone(),
                record,
            })
        })
        .collect()
}

pub fn parse_solve_commit_message(message: &str, source_order: usize) -> Option<SolveRecord> {
    let first_line = message.lines().next()?.trim();
    let captures = Regex::new(r"^solve\((?P<problem_id>[^)]+)\):\s*(?P<title>.*)$")
        .ok()?
        .captures(first_line)?;
    let problem_id = captures.name("problem_id")?.as_str().trim().to_string();
    if problem_id.is_empty() {
        return None;
    }

    let fields = message
        .lines()
        .skip(1)
        .filter_map(parse_message_field)
        .collect::<HashMap<_, _>>();

    Some(SolveRecord {
        problem_id,
        title: normalize_title_field(captures.name("title").map(|value| value.as_str())),
        verdict: normalize_stat_field(fields.get("Verdict").map(String::as_str)),
        difficulty: normalize_stat_field(fields.get("Difficulty").map(String::as_str)),
        tags: parse_tags(fields.get("Tags").map(String::as_str)),
        submission_id: parse_submission_id(fields.get("Submission-ID").map(String::as_str)),
        submission_time: fields
            .get("Submission-Time")
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok()),
        file_name: normalize_stat_field(fields.get("File").map(String::as_str)),
        source_order,
    })
}

pub fn summarize_solve_records(
    records: &[SolveRecord],
    algorithm_tag_names: Option<&HashSet<String>>,
) -> StatsSummary {
    let total_solve_records = records.len();
    let verdict_counts = build_count_distribution(
        records
            .iter()
            .map(|record| record.verdict.clone())
            .collect::<Vec<_>>(),
    );

    let mut unique_by_problem = BTreeMap::new();
    for record in records {
        unique_by_problem
            .entry(record.problem_id.clone())
            .and_modify(|current: &mut SolveRecord| {
                if solve_record_is_newer(record, current) {
                    *current = record.clone();
                }
            })
            .or_insert_with(|| record.clone());
    }

    let unique_records = unique_by_problem.into_values().collect::<Vec<_>>();
    let unique_problem_count = unique_records.len();
    let unique_ac_count = unique_records
        .iter()
        .filter(|record| record.verdict.eq_ignore_ascii_case("AC"))
        .count();
    let unique_non_ac_count = unique_problem_count.saturating_sub(unique_ac_count);
    let difficulty_counts = build_count_distribution(
        unique_records
            .iter()
            .map(|record| record.difficulty.clone())
            .collect::<Vec<_>>(),
    );
    let tag_counts = build_count_distribution(
        unique_records
            .iter()
            .flat_map(|record| {
                let filtered = filter_algorithm_tags(&record.tags, algorithm_tag_names);
                if filtered.is_empty() {
                    vec!["-".to_string()]
                } else {
                    filtered
                }
            })
            .collect::<Vec<_>>(),
    );

    StatsSummary {
        total_solve_records,
        unique_problem_count,
        unique_ac_count,
        unique_non_ac_count,
        verdict_counts,
        difficulty_counts,
        tag_counts,
    }
}

pub fn latest_records_by_file(records: &[HistoricalSolveRecord]) -> Vec<FileRecordSummary> {
    let mut latest = BTreeMap::new();
    for entry in records {
        latest
            .entry(entry.record.file_name.clone())
            .and_modify(|current: &mut HistoricalSolveRecord| {
                if solve_record_is_newer(&entry.record, &current.record) {
                    *current = entry.clone();
                }
            })
            .or_insert_with(|| entry.clone());
    }

    let mut summaries = latest
        .into_values()
        .map(|entry| FileRecordSummary {
            revision: entry.revision,
            problem_id: entry.record.problem_id,
            title: entry.record.title,
            file_name: entry.record.file_name,
            verdict: entry.record.verdict,
            difficulty: entry.record.difficulty,
            submission_id: entry.record.submission_id,
            submission_time: entry.record.submission_time,
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    summaries
}

fn filter_algorithm_tags(
    tags: &[String],
    algorithm_tag_names: Option<&HashSet<String>>,
) -> Vec<String> {
    match algorithm_tag_names {
        Some(names) => tags
            .iter()
            .filter(|tag| names.contains(tag.as_str()))
            .cloned()
            .collect(),
        None => tags.to_vec(),
    }
}

fn parse_message_field(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), value.trim().to_string()))
}

fn normalize_stat_field(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
        .to_string()
}

fn normalize_title_field(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Unknown Problem")
        .to_string()
}

fn parse_tags(value: Option<&str>) -> Vec<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_submission_id(value: Option<&str>) -> Option<u64> {
    value.and_then(|item| item.trim().parse::<u64>().ok())
}

fn build_count_distribution(values: Vec<String>) -> Vec<(String, usize)> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    let mut pairs = counts.into_iter().collect::<Vec<_>>();
    pairs.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    pairs
}

fn solve_record_is_newer(candidate: &SolveRecord, current: &SolveRecord) -> bool {
    match (candidate.submission_time, current.submission_time) {
        (Some(left), Some(right)) => left > right,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => candidate.source_order < current.source_order,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use chrono::{FixedOffset, TimeZone};

    use super::{
        FileRecordSummary, HistoricalSolveRecord, ProblemMetadata, SolveRecord, StatsSummary,
        SubmissionRecord, SyncSelection, build_commit_message, build_solve_commit_message,
        latest_records_by_file, parse_historical_solve_records, parse_solve_commit_message,
        parse_solve_records, summarize_solve_records,
    };

    fn sample_metadata() -> ProblemMetadata {
        ProblemMetadata {
            id: "P1001".to_string(),
            title: "A+B Problem".to_string(),
            difficulty: Some("入门".to_string()),
            tags: vec!["模拟".to_string(), "入门".to_string()],
            source: Some("Luogu".to_string()),
            url: "https://www.luogu.com.cn/problem/P1001".to_string(),
            fetched_at: FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                .single()
                .unwrap(),
        }
    }

    fn sample_record() -> SubmissionRecord {
        SubmissionRecord {
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
        }
    }

    #[test]
    fn build_commit_message_uses_submission_metadata_for_solve_commit() {
        let metadata = sample_metadata();
        let selection = SyncSelection::Submission(sample_record());

        let message = build_commit_message("P1001", "P1001.cpp", Some(&metadata), &selection);

        assert!(message.starts_with("solve(P1001): A+B Problem"));
        assert!(message.contains("Verdict: AC"));
        assert!(message.contains("Submission-ID: 123456"));
        assert!(message.contains("File: P1001.cpp"));
    }

    #[test]
    fn build_solve_commit_message_reuses_shared_solve_builder() {
        let metadata = sample_metadata();
        let record = sample_record();

        let message = build_solve_commit_message("P1001", "P1001.cpp", Some(&metadata), &record);

        assert!(message.starts_with("solve(P1001): A+B Problem"));
        assert!(message.contains("Submission-ID: 123456"));
    }

    #[test]
    fn build_commit_message_builds_chore_only_for_explicit_chore_selection() {
        let metadata = sample_metadata();

        let message =
            build_commit_message("P1001", "P1001.cpp", Some(&metadata), &SyncSelection::Chore);

        assert_eq!(message, "chore(P1001): 本地修改\n\nFile: P1001.cpp");
    }

    #[test]
    fn build_commit_message_builds_delete_commit_with_problem_context() {
        let metadata = sample_metadata();

        let message = build_commit_message(
            "P1001",
            "P1001.cpp",
            Some(&metadata),
            &SyncSelection::Delete,
        );

        assert_eq!(
            message,
            "remove(P1001): 删除题解文件\n\nTitle: A+B Problem\nFile: P1001.cpp"
        );
    }

    #[test]
    fn build_commit_message_returns_empty_for_skip() {
        let metadata = sample_metadata();

        let message =
            build_commit_message("P1001", "P1001.cpp", Some(&metadata), &SyncSelection::Skip);

        assert!(message.is_empty());
    }

    #[test]
    fn parse_solve_commit_message_extracts_stats_fields() {
        let message = "solve(P1001): A+B Problem\n\nVerdict: AC\nScore: 100\nTime: 50ms\nMemory: 1.2MB\nSubmission-ID: 123456\nSubmission-Time: 2024-01-15T14:32:00+08:00\nTags: 模拟, 入门\nDifficulty: 入门\nSource: Luogu\nFile: P1001.cpp";

        let record = parse_solve_commit_message(message, 0).unwrap();

        assert_eq!(record.problem_id, "P1001");
        assert_eq!(record.title, "A+B Problem");
        assert_eq!(record.verdict, "AC");
        assert_eq!(record.difficulty, "入门");
        assert_eq!(record.tags, vec!["模拟".to_string(), "入门".to_string()]);
        assert_eq!(record.submission_id, Some(123456));
        assert_eq!(record.file_name, "P1001.cpp");
        assert_eq!(
            record.submission_time,
            Some(
                FixedOffset::east_opt(8 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                    .single()
                    .unwrap()
            )
        );
    }

    #[test]
    fn parse_solve_commit_message_ignores_non_solve_commits() {
        assert!(parse_solve_commit_message("chore(P1001): 本地修改", 0).is_none());
        assert!(parse_solve_commit_message("random text", 0).is_none());
    }

    #[test]
    fn parse_solve_commit_message_uses_placeholders_for_missing_fields() {
        let message = "solve(P1001): A+B Problem\n\nFile: P1001.cpp";

        let record = parse_solve_commit_message(message, 0).unwrap();

        assert_eq!(record.verdict, "-");
        assert_eq!(record.difficulty, "-");
        assert!(record.tags.is_empty());
    }

    #[test]
    fn parse_solve_records_keeps_only_recognized_solve_messages() {
        let messages = vec![
            "solve(P1001): A".to_string(),
            "chore(P1001): 本地修改".to_string(),
            "solve(P1002): B".to_string(),
        ];

        let records = parse_solve_records(&messages);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].problem_id, "P1001");
        assert_eq!(records[1].problem_id, "P1002");
    }

    #[test]
    fn parse_historical_solve_records_keeps_revision_ids() {
        let entries = vec![(
            "abc123".to_string(),
            "solve(P1001): A+B Problem\n\nSubmission-ID: 42\nFile: P1001.cpp".to_string(),
        )];

        let records = parse_historical_solve_records(&entries);

        assert_eq!(
            records,
            vec![HistoricalSolveRecord {
                revision: "abc123".to_string(),
                record: SolveRecord {
                    problem_id: "P1001".to_string(),
                    title: "A+B Problem".to_string(),
                    verdict: "-".to_string(),
                    difficulty: "-".to_string(),
                    tags: Vec::new(),
                    submission_id: Some(42),
                    submission_time: None,
                    file_name: "P1001.cpp".to_string(),
                    source_order: 0,
                },
            }]
        );
    }

    #[test]
    fn summarize_solve_records_counts_total_and_unique_views() {
        let records = vec![
            SolveRecord {
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                verdict: "WA".to_string(),
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string()],
                submission_id: Some(1),
                submission_time: Some(
                    FixedOffset::east_opt(8 * 3600)
                        .unwrap()
                        .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                        .single()
                        .unwrap(),
                ),
                file_name: "P1001.cpp".to_string(),
                source_order: 1,
            },
            SolveRecord {
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                verdict: "AC".to_string(),
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string(), "二分".to_string()],
                submission_id: Some(2),
                submission_time: Some(
                    FixedOffset::east_opt(8 * 3600)
                        .unwrap()
                        .with_ymd_and_hms(2024, 1, 16, 14, 32, 0)
                        .single()
                        .unwrap(),
                ),
                file_name: "P1001.cpp".to_string(),
                source_order: 0,
            },
            SolveRecord {
                problem_id: "P1002".to_string(),
                title: "B".to_string(),
                verdict: "-".to_string(),
                difficulty: "-".to_string(),
                tags: Vec::new(),
                submission_id: None,
                submission_time: None,
                file_name: "P1002.cpp".to_string(),
                source_order: 2,
            },
        ];

        let algorithm_tags = HashSet::from([
            "模拟".to_string(),
            "二分".to_string(),
            "动态规划 DP".to_string(),
        ]);
        let summary = summarize_solve_records(&records, Some(&algorithm_tags));

        assert_eq!(
            summary,
            StatsSummary {
                total_solve_records: 3,
                unique_problem_count: 2,
                unique_ac_count: 1,
                unique_non_ac_count: 1,
                verdict_counts: vec![
                    ("-".to_string(), 1),
                    ("AC".to_string(), 1),
                    ("WA".to_string(), 1)
                ],
                difficulty_counts: vec![("-".to_string(), 1), ("入门".to_string(), 1)],
                tag_counts: vec![
                    ("-".to_string(), 1),
                    ("二分".to_string(), 1),
                    ("模拟".to_string(), 1),
                ],
            }
        );
    }

    #[test]
    fn summarize_solve_records_falls_back_to_traversal_order_without_time() {
        let older = SolveRecord {
            problem_id: "P1001".to_string(),
            title: "A".to_string(),
            verdict: "WA".to_string(),
            difficulty: "入门".to_string(),
            tags: vec!["模拟".to_string()],
            submission_id: Some(1),
            submission_time: None,
            file_name: "P1001.cpp".to_string(),
            source_order: 3,
        };
        let newer = SolveRecord {
            problem_id: "P1001".to_string(),
            title: "A".to_string(),
            verdict: "AC".to_string(),
            difficulty: "普及-".to_string(),
            tags: vec!["二分".to_string()],
            submission_id: Some(2),
            submission_time: None,
            file_name: "P1001.cpp".to_string(),
            source_order: 1,
        };

        let algorithm_tags = HashSet::from(["二分".to_string()]);
        let summary = summarize_solve_records(&[older, newer], Some(&algorithm_tags));

        assert_eq!(summary.unique_ac_count, 1);
        assert_eq!(summary.unique_non_ac_count, 0);
        assert_eq!(summary.difficulty_counts, vec![("普及-".to_string(), 1)]);
        assert_eq!(summary.tag_counts, vec![("二分".to_string(), 1)]);
    }

    #[test]
    fn summarize_solve_records_filters_non_algorithm_tags_only_in_stats() {
        let algorithm_tags = HashSet::from(["模拟".to_string()]);
        let records = vec![SolveRecord {
            problem_id: "P1001".to_string(),
            title: "A".to_string(),
            verdict: "AC".to_string(),
            difficulty: "入门".to_string(),
            tags: vec![
                "模拟".to_string(),
                "2024".to_string(),
                "NOIP 普及组".to_string(),
            ],
            submission_id: Some(1),
            submission_time: None,
            file_name: "P1001.cpp".to_string(),
            source_order: 0,
        }];

        let summary = summarize_solve_records(&records, Some(&algorithm_tags));

        assert_eq!(summary.tag_counts, vec![("模拟".to_string(), 1)]);
        assert_eq!(
            records[0].tags,
            vec![
                "模拟".to_string(),
                "2024".to_string(),
                "NOIP 普及组".to_string()
            ]
        );
    }

    #[test]
    fn latest_records_by_file_uses_latest_record_per_path() {
        let records = vec![
            HistoricalSolveRecord {
                revision: "old".to_string(),
                record: SolveRecord {
                    problem_id: "P1001".to_string(),
                    title: "A".to_string(),
                    verdict: "WA".to_string(),
                    difficulty: "入门".to_string(),
                    tags: vec!["模拟".to_string()],
                    submission_id: Some(1),
                    submission_time: Some(
                        FixedOffset::east_opt(8 * 3600)
                            .unwrap()
                            .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                            .single()
                            .unwrap(),
                    ),
                    file_name: "solutions/P1001.cpp".to_string(),
                    source_order: 1,
                },
            },
            HistoricalSolveRecord {
                revision: "new".to_string(),
                record: SolveRecord {
                    problem_id: "P1001".to_string(),
                    title: "A".to_string(),
                    verdict: "AC".to_string(),
                    difficulty: "入门".to_string(),
                    tags: vec!["模拟".to_string()],
                    submission_id: Some(2),
                    submission_time: Some(
                        FixedOffset::east_opt(8 * 3600)
                            .unwrap()
                            .with_ymd_and_hms(2024, 1, 16, 14, 32, 0)
                            .single()
                            .unwrap(),
                    ),
                    file_name: "solutions/P1001.cpp".to_string(),
                    source_order: 0,
                },
            },
        ];

        assert_eq!(
            latest_records_by_file(&records),
            vec![FileRecordSummary {
                revision: "new".to_string(),
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                file_name: "solutions/P1001.cpp".to_string(),
                verdict: "AC".to_string(),
                difficulty: "入门".to_string(),
                submission_id: Some(2),
                submission_time: Some(
                    FixedOffset::east_opt(8 * 3600)
                        .unwrap()
                        .with_ymd_and_hms(2024, 1, 16, 14, 32, 0)
                        .single()
                        .unwrap(),
                ),
            }]
        );
    }
}
