use std::collections::HashMap;

use chrono::DateTime;
use regex::Regex;

use crate::domain::{
    problem::ProblemMetadata,
    record::{HistoricalSolveRecord, SolveRecord, SyncSelection, TrainingFields},
    submission::SubmissionRecord,
    training_fields::format_training_fields,
};
use crate::problem::{normalize_problem_id_with_source, provider_from_problem_id, provider_label};

pub fn build_commit_message(
    problem_id: &str,
    file_name: &str,
    metadata: Option<&ProblemMetadata>,
    selection: &SyncSelection,
) -> String {
    match selection {
        SyncSelection::Submission(record) => {
            build_solve_commit_message(problem_id, file_name, metadata, record)
        }
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
    build_solve_commit_message_with_training(
        problem_id,
        file_name,
        metadata,
        record,
        &TrainingFields::default(),
    )
}

pub fn build_solve_commit_message_with_training(
    problem_id: &str,
    file_name: &str,
    metadata: Option<&ProblemMetadata>,
    record: &SubmissionRecord,
    training: &TrainingFields,
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
        .map(|item| provider_label(item.provider))
        .or_else(|| metadata.and_then(|item| item.source.as_deref()))
        .unwrap_or_else(|| provider_label(record.provider));
    let contest = metadata
        .and_then(|item| item.contest.as_deref())
        .filter(|value| !value.trim().is_empty());
    let submitted_at = record
        .submitted_at
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| "-".to_string());

    let mut lines = vec![
        format!("solve({problem_id}): {title}"),
        String::new(),
        format!("Verdict: {}", record.verdict),
        format!(
            "Score: {}",
            record
                .score
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "Time: {}",
            record
                .time_ms
                .map(|value| format!("{value}ms"))
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "Memory: {}",
            record
                .memory_mb
                .map(|value| format!("{value:.1}MB"))
                .unwrap_or_else(|| "-".to_string())
        ),
        format!("Submission-ID: {}", record.submission_id),
        format!("Submission-Time: {submitted_at}"),
        format!("Tags: {tags}"),
        format!("Difficulty: {difficulty}"),
        format!("Source: {source}"),
        format!("Contest: {}", contest.unwrap_or("-")),
        format!("File: {file_name}"),
    ];
    for (key, value) in format_training_fields(training) {
        lines.push(format!("{key}: {value}"));
    }
    lines.join("\n")
}

pub fn build_solve_record_message(record: &SolveRecord) -> String {
    let mut lines = vec![
        format!("solve({}): {}", record.problem_id, record.title),
        String::new(),
        format!("Verdict: {}", record.verdict),
        format!(
            "Score: {}",
            record
                .score
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "Time: {}",
            record
                .time_ms
                .map(|value| format!("{value}ms"))
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "Memory: {}",
            record
                .memory_mb
                .map(|value| format!("{value:.1}MB"))
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "Submission-ID: {}",
            record
                .submission_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "Submission-Time: {}",
            record
                .submission_time
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!(
            "Tags: {}",
            if record.tags.is_empty() {
                "-".to_string()
            } else {
                record.tags.join(", ")
            }
        ),
        format!("Difficulty: {}", record.difficulty),
        format!("Source: {}", record.source),
        format!("Contest: {}", record.contest.as_deref().unwrap_or("-")),
        format!("File: {}", record.file_name),
    ];
    for (key, value) in format_training_fields(&record.training) {
        lines.push(format!("{key}: {value}"));
    }
    lines.join("\n")
}

pub fn parse_solve_records(messages: &[String]) -> Vec<SolveRecord> {
    messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| parse_solve_commit_message(message, index))
        .collect()
}

pub fn parse_historical_solve_records(entries: &[(String, String)]) -> Vec<HistoricalSolveRecord> {
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
    let raw_problem_id = captures.name("problem_id")?.as_str().trim();
    if raw_problem_id.is_empty() {
        return None;
    }

    let fields = message
        .lines()
        .skip(1)
        .filter_map(parse_message_field)
        .collect::<HashMap<_, _>>();
    let source = normalize_stat_field(field_value(&fields, &["来源", "Source"]));
    let problem_id = normalize_problem_id_with_source(raw_problem_id, Some(source.as_str()));
    let provider = provider_from_problem_id(&problem_id);

    Some(SolveRecord {
        problem_id,
        provider,
        title: normalize_title_field(captures.name("title").map(|value| value.as_str())),
        verdict: normalize_stat_field(field_value(&fields, &["判题结果", "Verdict"])),
        score: parse_score(field_value(&fields, &["分数", "Score"])),
        time_ms: parse_time_ms(field_value(&fields, &["耗时", "Time"])),
        memory_mb: parse_memory_mb(field_value(&fields, &["内存", "Memory"])),
        difficulty: normalize_stat_field(field_value(&fields, &["难度", "Difficulty"])),
        tags: parse_tags(field_value(&fields, &["标签", "Tags"])),
        source,
        contest: normalize_optional_field(field_value(&fields, &["比赛", "Contest"])),
        submission_id: parse_submission_id(field_value(&fields, &["提交编号", "Submission-ID"])),
        submission_time: field_value(&fields, &["提交时间", "Submission-Time"])
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok()),
        file_name: normalize_stat_field(field_value(&fields, &["文件", "File"])),
        training: crate::domain::training_fields::parse_training_fields(&fields),
        source_order,
    })
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

fn normalize_optional_field(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .map(ToString::to_string)
}

fn field_value<'a>(fields: &'a HashMap<String, String>, aliases: &[&str]) -> Option<&'a str> {
    aliases
        .iter()
        .find_map(|alias| fields.get(*alias).map(String::as_str))
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

fn parse_score(value: Option<&str>) -> Option<i64> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() || trimmed == "-" {
            None
        } else {
            trimmed.parse::<i64>().ok()
        }
    })
}

fn parse_time_ms(value: Option<&str>) -> Option<u64> {
    value.and_then(|item| {
        let trimmed = item.trim();
        let trimmed = trimmed.strip_suffix("ms").unwrap_or(trimmed);
        if trimmed.is_empty() || trimmed == "-" {
            None
        } else {
            trimmed.parse::<u64>().ok()
        }
    })
}

fn parse_memory_mb(value: Option<&str>) -> Option<f64> {
    value.and_then(|item| {
        let trimmed = item.trim();
        let trimmed = trimmed.strip_suffix("MB").unwrap_or(trimmed);
        if trimmed.is_empty() || trimmed == "-" {
            None
        } else {
            trimmed.parse::<f64>().ok()
        }
    })
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};

    use super::{
        build_commit_message, build_solve_commit_message, build_solve_commit_message_with_training,
        build_solve_record_message, parse_historical_solve_records, parse_solve_commit_message,
        parse_solve_records,
    };
    use crate::domain::{
        problem::ProblemMetadata,
        record::{HistoricalSolveRecord, SolveRecord, SyncSelection, TrainingFields},
        submission::SubmissionRecord,
    };

    fn sample_metadata() -> ProblemMetadata {
        ProblemMetadata {
            id: "luogu:P1001".to_string(),
            provider: crate::problem::ProblemProvider::Luogu,
            title: "A+B Problem".to_string(),
            difficulty: Some("入门".to_string()),
            tags: vec!["模拟".to_string(), "入门".to_string()],
            source: Some("Luogu".to_string()),
            contest: None,
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
            problem_id: Some("luogu:P1001".to_string()),
            provider: crate::problem::ProblemProvider::Luogu,
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

        let message = build_commit_message("luogu:P1001", "P1001.cpp", Some(&metadata), &selection);

        assert!(message.starts_with("solve(luogu:P1001): A+B Problem"));
        assert!(message.contains("Verdict: AC"));
        assert!(message.contains("Submission-ID: 123456"));
        assert!(message.contains("File: P1001.cpp"));
    }

    #[test]
    fn build_solve_commit_message_reuses_shared_solve_builder() {
        let metadata = sample_metadata();
        let record = sample_record();

        let message =
            build_solve_commit_message("luogu:P1001", "P1001.cpp", Some(&metadata), &record);

        assert!(message.starts_with("solve(luogu:P1001): A+B Problem"));
        assert!(message.contains("Submission-ID: 123456"));
    }

    #[test]
    fn build_commit_message_builds_delete_commit_with_problem_context() {
        let metadata = sample_metadata();

        let message = build_commit_message(
            "luogu:P1001",
            "P1001.cpp",
            Some(&metadata),
            &SyncSelection::Delete,
        );

        assert_eq!(
            message,
            "remove(luogu:P1001): 删除题解文件\n\nTitle: A+B Problem\nFile: P1001.cpp"
        );
    }

    #[test]
    fn parse_solve_commit_message_extracts_stats_fields() {
        let message = "solve(P1001): A+B Problem\n\nVerdict: AC\nScore: 100\nTime: 50ms\nMemory: 1.2MB\nSubmission-ID: 123456\nSubmission-Time: 2024-01-15T14:32:00+08:00\nTags: 模拟, 入门\nDifficulty: 入门\nSource: Luogu\nFile: P1001.cpp";

        let record = parse_solve_commit_message(message, 0).unwrap();

        assert_eq!(record.problem_id, "luogu:P1001");
        assert_eq!(record.title, "A+B Problem");
        assert_eq!(record.verdict, "AC");
        assert_eq!(record.score, Some(100));
        assert_eq!(record.time_ms, Some(50));
        assert_eq!(record.memory_mb, Some(1.2));
        assert_eq!(record.difficulty, "入门");
        assert_eq!(record.tags, vec!["模拟".to_string(), "入门".to_string()]);
        assert_eq!(record.source, "Luogu");
        assert_eq!(record.submission_id, Some(123456));
        assert_eq!(record.file_name, "P1001.cpp");
    }

    #[test]
    fn solve_commit_message_round_trips_training_fields() {
        let metadata = sample_metadata();
        let record = sample_record();
        let training = TrainingFields {
            note: Some("先枚举后贪心".to_string()),
            confidence: Some("medium".to_string()),
            ..TrainingFields::default()
        };

        let message = build_solve_commit_message_with_training(
            "luogu:P1001",
            "P1001.cpp",
            Some(&metadata),
            &record,
            &training,
        );
        let parsed = parse_solve_commit_message(&message, 0).unwrap();

        assert_eq!(parsed.training, training);
    }

    #[test]
    fn build_solve_record_message_preserves_existing_record_data() {
        let record = SolveRecord {
            problem_id: "luogu:P1001".to_string(),
            provider: crate::problem::ProblemProvider::Luogu,
            title: "A+B Problem".to_string(),
            verdict: "AC".to_string(),
            score: Some(100),
            time_ms: Some(50),
            memory_mb: Some(1.2),
            difficulty: "入门".to_string(),
            tags: vec!["模拟".to_string()],
            source: "Luogu".to_string(),
            contest: None,
            submission_id: Some(123456),
            submission_time: Some(
                FixedOffset::east_opt(8 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                    .single()
                    .unwrap(),
            ),
            file_name: "P1001.cpp".to_string(),
            training: TrainingFields {
                note: Some("复习".to_string()),
                ..TrainingFields::default()
            },
            source_order: 0,
        };

        let message = build_solve_record_message(&record);
        assert!(message.contains("Score: 100"));
        assert!(message.contains("Note: 复习"));
    }

    #[test]
    fn parse_solve_records_and_history_keep_only_solve_commits() {
        let messages = vec![
            "solve(P1001): A".to_string(),
            "chore(P1001): 本地修改".to_string(),
            "solve(P1002): B".to_string(),
        ];
        let records = parse_solve_records(&messages);
        assert_eq!(records.len(), 2);

        let entries = vec![(
            "abc123".to_string(),
            "solve(P1001): A+B Problem\n\nSubmission-ID: 42\nFile: P1001.cpp".to_string(),
        )];
        let history = parse_historical_solve_records(&entries);
        assert_eq!(
            history,
            vec![HistoricalSolveRecord {
                revision: "abc123".to_string(),
                record: SolveRecord {
                    problem_id: "luogu:P1001".to_string(),
                    provider: crate::problem::ProblemProvider::Luogu,
                    title: "A+B Problem".to_string(),
                    verdict: "-".to_string(),
                    score: None,
                    time_ms: None,
                    memory_mb: None,
                    difficulty: "-".to_string(),
                    tags: Vec::new(),
                    source: "-".to_string(),
                    contest: None,
                    submission_id: Some(42),
                    submission_time: None,
                    file_name: "P1001.cpp".to_string(),
                    training: TrainingFields::default(),
                    source_order: 0,
                },
            }]
        );
    }
}
