use std::collections::{BTreeMap, HashSet};

use crate::domain::record::{FileRecordSummary, HistoricalSolveRecord, SolveRecord};

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

    use super::{StatsSummary, latest_records_by_file, summarize_solve_records};
    use crate::domain::record::{FileRecordSummary, HistoricalSolveRecord, SolveRecord};

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
