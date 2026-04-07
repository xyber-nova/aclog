use std::collections::HashMap;

use chrono::DateTime;
use regex::Regex;

use crate::domain::{
    problem::ProblemMetadata,
    record::{HistoricalSolveRecord, SolveRecord, SyncSelection},
    submission::SubmissionRecord,
};

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

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};

    use super::{
        build_commit_message, build_solve_commit_message, parse_historical_solve_records,
        parse_solve_commit_message, parse_solve_records,
    };
    use crate::domain::{
        problem::ProblemMetadata,
        record::{HistoricalSolveRecord, SolveRecord, SyncSelection},
        submission::SubmissionRecord,
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
}
