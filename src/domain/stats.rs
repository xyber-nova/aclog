use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Duration, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    record::{FileRecordSummary, HistoricalSolveRecord, SolveRecord},
    record_index::RecordIndex,
};
use crate::utils::{is_ac_verdict, normalize_verdict};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsSummary {
    pub total_solve_records: usize,
    pub unique_problem_count: usize,
    pub unique_ac_count: usize,
    pub unique_non_ac_count: usize,
    pub first_ac_count: usize,
    pub repeated_practice_count: usize,
    pub time_window_days: Option<i64>,
    pub verdict_counts: Vec<(String, usize)>,
    pub difficulty_counts: Vec<(String, usize)>,
    pub tag_counts: Vec<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCandidate {
    pub kind: String,
    pub label: String,
    pub problem_id: Option<String>,
    pub title: Option<String>,
    pub verdict: Option<String>,
    pub last_submission_time: Option<DateTime<FixedOffset>>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsDashboard {
    pub summary: StatsSummary,
    pub review_candidates: Vec<ReviewCandidate>,
    pub start_in_review: bool,
}

pub fn summarize_solve_records(
    records: &[SolveRecord],
    algorithm_tag_names: Option<&HashSet<String>>,
) -> StatsSummary {
    summarize_solve_records_with_window(records, None, algorithm_tag_names)
}

pub fn summarize_solve_records_with_window(
    records: &[SolveRecord],
    window_days: Option<i64>,
    algorithm_tag_names: Option<&HashSet<String>>,
) -> StatsSummary {
    let cutoff = window_days.map(|days| Utc::now() - Duration::days(days));
    let window_records = records
        .iter()
        .filter(|record| is_within_window(record, cutoff))
        .cloned()
        .collect::<Vec<_>>();
    let activity_records = if window_days.is_some() {
        &window_records
    } else {
        records
    };
    let total_solve_records = activity_records.len();
    let verdict_counts = build_count_distribution(
        activity_records
            .iter()
            .map(|record| normalize_verdict(&record.verdict).into_owned())
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

    let active_problem_ids = if window_days.is_some() {
        activity_records
            .iter()
            .map(|record| record.problem_id.clone())
            .collect::<BTreeSet<_>>()
    } else {
        unique_by_problem.keys().cloned().collect::<BTreeSet<_>>()
    };
    let unique_records = unique_by_problem
        .into_iter()
        .filter_map(|(problem_id, record)| {
            if active_problem_ids.contains(&problem_id) {
                Some(record)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let unique_problem_count = unique_records.len();
    let unique_ac_count = unique_records
        .iter()
        .filter(|record| is_ac_verdict(&record.verdict))
        .count();
    let unique_non_ac_count = unique_problem_count.saturating_sub(unique_ac_count);
    let first_ac_count = count_first_acs(records, &active_problem_ids, cutoff);
    let repeated_practice_count = active_problem_ids
        .iter()
        .filter(|problem_id| {
            records
                .iter()
                .filter(|record| &record.problem_id == *problem_id)
                .count()
                > 1
        })
        .count();
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
        first_ac_count,
        repeated_practice_count,
        time_window_days: window_days,
        verdict_counts,
        difficulty_counts,
        tag_counts,
    }
}

pub fn build_review_candidates(
    records: &[SolveRecord],
    window_days: Option<i64>,
    algorithm_tag_names: Option<&HashSet<String>>,
) -> Vec<ReviewCandidate> {
    let cutoff = window_days.map(|days| Utc::now() - Duration::days(days));
    let stale_cutoff = cutoff.unwrap_or_else(|| Utc::now() - Duration::days(30));
    let index = build_problem_histories(records);
    let mut candidates = Vec::new();

    for (problem_id, history) in &index {
        let latest = history.last().expect("history should not be empty");
        if let Some(submission_time) = latest.submission_time {
            if submission_time.with_timezone(&Utc) < stale_cutoff {
                candidates.push(ReviewCandidate {
                    kind: "stale".to_string(),
                    label: problem_id.clone(),
                    problem_id: Some(problem_id.clone()),
                    title: Some(latest.title.clone()),
                    verdict: Some(normalize_verdict(&latest.verdict).into_owned()),
                    last_submission_time: latest.submission_time,
                    reason: format!(
                        "距离上次练习已超过 {} 天",
                        (Utc::now() - submission_time.with_timezone(&Utc)).num_days()
                    ),
                });
            }
        }

        let non_ac_attempts = history
            .iter()
            .filter(|record| !is_ac_verdict(&record.verdict))
            .count();
        if !is_ac_verdict(&latest.verdict)
            || non_ac_attempts >= 2
            || latest.training.mistakes.is_some()
        {
            let reason = if !is_ac_verdict(&latest.verdict) {
                format!("最近状态仍为 {}", normalize_verdict(&latest.verdict))
            } else if non_ac_attempts >= 2 {
                format!("历史里至少有 {} 次非 AC 尝试", non_ac_attempts)
            } else {
                "记录中包含待复盘的错因".to_string()
            };
            candidates.push(ReviewCandidate {
                kind: "retry".to_string(),
                label: problem_id.clone(),
                problem_id: Some(problem_id.clone()),
                title: Some(latest.title.clone()),
                verdict: Some(normalize_verdict(&latest.verdict).into_owned()),
                last_submission_time: latest.submission_time,
                reason,
            });
        }
    }

    let activity_records = records
        .iter()
        .filter(|record| is_within_window(record, cutoff))
        .collect::<Vec<_>>();
    let activity_records = if window_days.is_some() {
        activity_records
    } else {
        records.iter().collect::<Vec<_>>()
    };
    let mut weakness_scores: HashMap<String, usize> = HashMap::new();
    for record in activity_records {
        let filtered_tags = filter_algorithm_tags(&record.tags, algorithm_tag_names);
        for tag in filtered_tags {
            let mut score = 0;
            if !is_ac_verdict(&record.verdict) {
                score += 1;
            }
            if record
                .training
                .confidence
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case("low"))
            {
                score += 1;
            }
            if score > 0 {
                *weakness_scores.entry(tag).or_insert(0) += score;
            }
        }
    }
    let mut weakness_tags = weakness_scores.into_iter().collect::<Vec<_>>();
    weakness_tags.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (tag, score) in weakness_tags {
        candidates.push(ReviewCandidate {
            kind: "weakness".to_string(),
            label: tag.clone(),
            problem_id: None,
            title: None,
            verdict: None,
            last_submission_time: None,
            reason: format!("最近窗口内该标签累计出现 {} 次非 AC 或低熟练度信号", score),
        });
    }

    candidates.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| right.last_submission_time.cmp(&left.last_submission_time))
    });
    candidates
}

pub fn latest_records_by_file(records: &[HistoricalSolveRecord]) -> Vec<FileRecordSummary> {
    RecordIndex::build(records).current_by_file().to_vec()
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

fn build_problem_histories(records: &[SolveRecord]) -> BTreeMap<String, Vec<SolveRecord>> {
    let mut histories = BTreeMap::new();
    for record in records {
        histories
            .entry(record.problem_id.clone())
            .or_insert_with(Vec::new)
            .push(record.clone());
    }
    for history in histories.values_mut() {
        history.sort_by(
            |left, right| match (left.submission_time, right.submission_time) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => right.source_order.cmp(&left.source_order),
            },
        );
    }
    histories
}

fn count_first_acs(
    records: &[SolveRecord],
    active_problem_ids: &BTreeSet<String>,
    cutoff: Option<DateTime<Utc>>,
) -> usize {
    let histories = build_problem_histories(records);
    active_problem_ids
        .iter()
        .filter(|problem_id| {
            histories
                .get(problem_id.as_str())
                .and_then(|history| history.iter().find(|record| is_ac_verdict(&record.verdict)))
                .is_some_and(|record| is_within_window(record, cutoff))
        })
        .count()
}

fn is_within_window(record: &SolveRecord, cutoff: Option<DateTime<Utc>>) -> bool {
    match cutoff {
        Some(cutoff) => record
            .submission_time
            .map(|value| value.with_timezone(&Utc) >= cutoff)
            .unwrap_or(false),
        None => true,
    }
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
        StatsSummary, build_review_candidates, latest_records_by_file, summarize_solve_records,
    };
    use crate::domain::record::{
        FileRecordSummary, HistoricalSolveRecord, SolveRecord, TrainingFields,
    };

    #[test]
    fn summarize_solve_records_counts_total_and_unique_views() {
        let records = vec![
            SolveRecord {
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                verdict: "WA".to_string(),
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
                        .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                        .single()
                        .unwrap(),
                ),
                file_name: "P1001.cpp".to_string(),
                training: TrainingFields::default(),
                source_order: 1,
            },
            SolveRecord {
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                verdict: "AC".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string(), "二分".to_string()],
                source: "Luogu".to_string(),
                submission_id: Some(2),
                submission_time: Some(
                    FixedOffset::east_opt(8 * 3600)
                        .unwrap()
                        .with_ymd_and_hms(2024, 1, 16, 14, 32, 0)
                        .single()
                        .unwrap(),
                ),
                file_name: "P1001.cpp".to_string(),
                training: TrainingFields::default(),
                source_order: 0,
            },
            SolveRecord {
                problem_id: "P1002".to_string(),
                title: "B".to_string(),
                verdict: "-".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "-".to_string(),
                tags: Vec::new(),
                source: "-".to_string(),
                submission_id: None,
                submission_time: None,
                file_name: "P1002.cpp".to_string(),
                training: TrainingFields::default(),
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
                first_ac_count: 1,
                repeated_practice_count: 1,
                time_window_days: None,
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
    fn build_review_candidates_returns_stale_retry_and_weakness_entries() {
        let records = vec![
            SolveRecord {
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                verdict: "WA".to_string(),
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
                    mistakes: Some("边界错".to_string()),
                    confidence: Some("low".to_string()),
                    ..TrainingFields::default()
                },
                source_order: 1,
            },
            SolveRecord {
                problem_id: "P1002".to_string(),
                title: "B".to_string(),
                verdict: "AC".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "普及-".to_string(),
                tags: vec!["二分".to_string()],
                source: "Luogu".to_string(),
                submission_id: Some(2),
                submission_time: Some(
                    FixedOffset::east_opt(8 * 3600)
                        .unwrap()
                        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
                        .single()
                        .unwrap(),
                ),
                file_name: "P1002.cpp".to_string(),
                training: TrainingFields::default(),
                source_order: 0,
            },
        ];
        let algorithm_tags = HashSet::from(["模拟".to_string(), "二分".to_string()]);

        let candidates = build_review_candidates(&records, None, Some(&algorithm_tags));

        assert!(
            candidates.iter().any(|item| {
                item.kind == "retry" && item.problem_id.as_deref() == Some("P1001")
            })
        );
        assert!(
            candidates.iter().any(|item| {
                item.kind == "stale" && item.problem_id.as_deref() == Some("P1001")
            })
        );
        assert!(
            candidates
                .iter()
                .any(|item| item.kind == "weakness" && item.label == "模拟")
        );
        assert!(!candidates.is_empty());
        let _json_like = serde_json::to_string(&candidates).unwrap();
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
                            .with_ymd_and_hms(2024, 1, 15, 14, 32, 0)
                            .single()
                            .unwrap(),
                    ),
                    file_name: "solutions/P1001.cpp".to_string(),
                    training: TrainingFields::default(),
                    source_order: 1,
                },
            },
            HistoricalSolveRecord {
                revision: "new".to_string(),
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
                    submission_id: Some(2),
                    submission_time: Some(
                        FixedOffset::east_opt(8 * 3600)
                            .unwrap()
                            .with_ymd_and_hms(2024, 1, 16, 14, 32, 0)
                            .single()
                            .unwrap(),
                    ),
                    file_name: "solutions/P1001.cpp".to_string(),
                    training: TrainingFields::default(),
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
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                source: "Luogu".to_string(),
                tags: vec!["模拟".to_string()],
                submission_id: Some(2),
                submission_time: Some(
                    FixedOffset::east_opt(8 * 3600)
                        .unwrap()
                        .with_ymd_and_hms(2024, 1, 16, 14, 32, 0)
                        .single()
                        .unwrap(),
                ),
                training: TrainingFields::default(),
            }]
        );
    }
}
