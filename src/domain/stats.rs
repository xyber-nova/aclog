use std::collections::{BTreeMap, BTreeSet, HashSet};

use chrono::{DateTime, Duration, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    browser::BrowserProviderView,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewSettings {
    pub problem_interval_days: i64,
    pub tag_window_days: i64,
    pub tag_target_problems: usize,
}

impl ReviewSettings {
    pub fn normalized(self) -> Self {
        Self {
            problem_interval_days: self.problem_interval_days.max(1),
            tag_window_days: self.tag_window_days.max(1),
            tag_target_problems: self.tag_target_problems.max(1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReviewSuggestions {
    pub problem_reviews: Vec<ProblemReviewCandidate>,
    pub tag_practice_suggestions: Vec<TagPracticeSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemReviewCandidate {
    pub problem_id: String,
    pub title: String,
    pub verdict: String,
    pub last_submission_time: Option<DateTime<FixedOffset>>,
    pub priority: i32,
    pub reasons: Vec<String>,
    pub matched_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagPracticeSuggestion {
    pub tag: String,
    pub recent_unique_problems: usize,
    pub lifetime_unique_problems: usize,
    pub priority: i32,
    pub reason: String,
    pub recent_unstable_signal_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsDashboard {
    pub summary: StatsSummary,
    pub problem_reviews: Vec<ProblemReviewCandidate>,
    pub tag_practice_suggestions: Vec<TagPracticeSuggestion>,
    pub start_in_review: bool,
    #[serde(default)]
    pub provider_dashboards: Vec<StatsProviderDashboard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsProviderDashboard {
    pub provider: BrowserProviderView,
    pub summary: StatsSummary,
    pub problem_reviews: Vec<ProblemReviewCandidate>,
    pub tag_practice_suggestions: Vec<TagPracticeSuggestion>,
    pub tag_features_supported: bool,
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

pub fn build_review_suggestions(
    records: &[SolveRecord],
    settings: ReviewSettings,
    algorithm_tag_names: Option<&HashSet<String>>,
) -> ReviewSuggestions {
    let settings = settings.normalized();
    ReviewSuggestions {
        problem_reviews: build_problem_review_candidates(records, settings, algorithm_tag_names),
        tag_practice_suggestions: build_tag_practice_suggestions(
            records,
            settings,
            algorithm_tag_names,
        ),
    }
}

pub fn latest_records_by_file(records: &[HistoricalSolveRecord]) -> Vec<FileRecordSummary> {
    RecordIndex::build(records).current_by_file().to_vec()
}

fn build_problem_review_candidates(
    records: &[SolveRecord],
    settings: ReviewSettings,
    algorithm_tag_names: Option<&HashSet<String>>,
) -> Vec<ProblemReviewCandidate> {
    let histories = build_problem_histories(records);
    let mut candidates = Vec::new();

    for (problem_id, history) in &histories {
        let latest = history.last().expect("history should not be empty");
        let mut priority = 0;
        let mut reasons = Vec::new();

        if !is_ac_verdict(&latest.verdict) {
            priority += 6;
            reasons.push(format!(
                "最近状态仍为 {}",
                normalize_verdict(&latest.verdict)
            ));
        }
        if latest.training.mistakes.is_some() {
            priority += 3;
            reasons.push("记录中包含待复盘错因".to_string());
        }
        if latest
            .training
            .confidence
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("low"))
        {
            priority += 3;
            reasons.push("最近熟练度标记为 low".to_string());
        }

        let extra_non_ac_attempts = history
            .iter()
            .rev()
            .skip(1)
            .take(2)
            .filter(|record| !is_ac_verdict(&record.verdict))
            .count();
        if extra_non_ac_attempts > 0 {
            let bonus = (extra_non_ac_attempts as i32 * 2).min(4);
            priority += bonus;
            reasons.push(format!(
                "最近 3 次记录中还有 {} 次非 AC",
                extra_non_ac_attempts
            ));
        }

        if let Some(submission_time) = latest.submission_time {
            let elapsed_days = (Utc::now() - submission_time.with_timezone(&Utc)).num_days();
            let interval_bonus =
                review_interval_bonus(elapsed_days, settings.problem_interval_days);
            if interval_bonus > 0 {
                priority += interval_bonus;
                reasons.push(format!(
                    "已达到 {} 天复习间隔（距上次练习 {} 天）",
                    settings.problem_interval_days, elapsed_days
                ));
            }
        }

        if priority == 0 {
            continue;
        }

        candidates.push(ProblemReviewCandidate {
            problem_id: problem_id.clone(),
            title: latest.title.clone(),
            verdict: normalize_verdict(&latest.verdict).into_owned(),
            last_submission_time: latest.submission_time,
            priority,
            reasons,
            matched_tags: filter_algorithm_tags(&latest.tags, algorithm_tag_names),
        });
    }

    candidates.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.last_submission_time.cmp(&right.last_submission_time))
            .then_with(|| left.problem_id.cmp(&right.problem_id))
    });
    candidates
}

fn build_tag_practice_suggestions(
    records: &[SolveRecord],
    settings: ReviewSettings,
    algorithm_tag_names: Option<&HashSet<String>>,
) -> Vec<TagPracticeSuggestion> {
    let recent_cutoff = Utc::now() - Duration::days(settings.tag_window_days);
    let mut lifetime_unique_problems: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut recent_unique_problems: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut recent_unstable_signals: BTreeMap<String, usize> = BTreeMap::new();

    for record in records {
        let tags = filter_algorithm_tags(&record.tags, algorithm_tag_names);
        if tags.is_empty() {
            continue;
        }

        for tag in tags {
            lifetime_unique_problems
                .entry(tag.clone())
                .or_default()
                .insert(record.problem_id.clone());

            if is_within_window(record, Some(recent_cutoff)) {
                recent_unique_problems
                    .entry(tag.clone())
                    .or_default()
                    .insert(record.problem_id.clone());

                if !is_ac_verdict(&record.verdict)
                    || record
                        .training
                        .confidence
                        .as_deref()
                        .is_some_and(|value| value.eq_ignore_ascii_case("low"))
                {
                    *recent_unstable_signals.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    let mut suggestions = Vec::new();
    for (tag, lifetime_problems) in lifetime_unique_problems {
        let lifetime_count = lifetime_problems.len();
        let recent_count = recent_unique_problems
            .get(&tag)
            .map(BTreeSet::len)
            .unwrap_or_default();
        if recent_count >= settings.tag_target_problems {
            continue;
        }

        let unstable_count = recent_unstable_signals
            .get(&tag)
            .copied()
            .unwrap_or_default();
        let gap = settings.tag_target_problems.saturating_sub(recent_count);
        let priority = gap as i32 * 100 - (lifetime_count.min(99) as i32);
        let mut reason = format!(
            "最近 {} 天仅练过 {} 题，建议补样本",
            settings.tag_window_days, recent_count
        );
        if unstable_count > 0 {
            reason.push_str(&format!("；最近这类题也有 {} 次不稳信号", unstable_count));
        }

        suggestions.push(TagPracticeSuggestion {
            tag,
            recent_unique_problems: recent_count,
            lifetime_unique_problems: lifetime_count,
            priority,
            reason,
            recent_unstable_signal_count: unstable_count,
        });
    }

    suggestions.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.tag.cmp(&right.tag))
    });
    suggestions
}

fn review_interval_bonus(elapsed_days: i64, interval_days: i64) -> i32 {
    if elapsed_days < interval_days {
        0
    } else if elapsed_days >= interval_days * 3 {
        3
    } else if elapsed_days >= interval_days * 2 {
        2
    } else {
        1
    }
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

    use chrono::{FixedOffset, TimeZone, Utc};

    use super::{
        ProblemReviewCandidate, ReviewSettings, StatsSummary, TagPracticeSuggestion,
        build_review_suggestions, latest_records_by_file, summarize_solve_records,
    };
    use crate::domain::record::{
        FileRecordSummary, HistoricalSolveRecord, SolveRecord, TrainingFields,
    };

    #[test]
    fn summarize_solve_records_counts_total_and_unique_views() {
        let records = vec![
            SolveRecord {
                problem_id: "luogu:P1001".to_string(),
                provider: crate::problem::ProblemProvider::Luogu,
                title: "A".to_string(),
                verdict: "WA".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string()],
                source: "Luogu".to_string(),
                contest: None,
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
                problem_id: "luogu:P1001".to_string(),
                provider: crate::problem::ProblemProvider::Luogu,
                title: "A".to_string(),
                verdict: "AC".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string(), "二分".to_string()],
                source: "Luogu".to_string(),
                contest: None,
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
                problem_id: "unknown:P1002".to_string(),
                provider: crate::problem::ProblemProvider::Unknown,
                title: "B".to_string(),
                verdict: "-".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "-".to_string(),
                tags: Vec::new(),
                source: "-".to_string(),
                contest: None,
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
    fn build_review_suggestions_includes_immediate_problem_review() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
        let records = vec![SolveRecord {
            problem_id: "luogu:P1001".to_string(),
            provider: crate::problem::ProblemProvider::Luogu,
            title: "A".to_string(),
            verdict: "WA".to_string(),
            score: None,
            time_ms: None,
            memory_mb: None,
            difficulty: "入门".to_string(),
            tags: vec!["模拟".to_string()],
            source: "Luogu".to_string(),
            contest: None,
            submission_id: Some(1),
            submission_time: Some(now - chrono::Duration::days(1)),
            file_name: "P1001.cpp".to_string(),
            training: TrainingFields {
                mistakes: Some("边界".to_string()),
                confidence: Some("low".to_string()),
                ..TrainingFields::default()
            },
            source_order: 0,
        }];

        let suggestions = build_review_suggestions(
            &records,
            ReviewSettings {
                problem_interval_days: 21,
                tag_window_days: 60,
                tag_target_problems: 5,
            },
            Some(&HashSet::from(["模拟".to_string()])),
        );

        assert_eq!(suggestions.problem_reviews.len(), 1);
        let candidate = &suggestions.problem_reviews[0];
        assert_eq!(candidate.problem_id, "luogu:P1001");
        assert!(candidate.priority >= 12);
        assert!(
            candidate
                .reasons
                .iter()
                .any(|reason| reason.contains("最近状态仍为"))
        );
        assert!(
            candidate
                .reasons
                .iter()
                .any(|reason| reason.contains("待复盘错因"))
        );
        assert!(
            candidate
                .reasons
                .iter()
                .any(|reason| reason.contains("熟练度"))
        );
    }

    #[test]
    fn build_review_suggestions_adds_interval_bonus_for_stable_ac() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
        let records = vec![SolveRecord {
            problem_id: "luogu:P2001".to_string(),
            provider: crate::problem::ProblemProvider::Luogu,
            title: "Stable".to_string(),
            verdict: "AC".to_string(),
            score: None,
            time_ms: None,
            memory_mb: None,
            difficulty: "入门".to_string(),
            tags: vec!["二分".to_string()],
            source: "Luogu".to_string(),
            contest: None,
            submission_id: Some(2),
            submission_time: Some(now - chrono::Duration::days(50)),
            file_name: "P2001.cpp".to_string(),
            training: TrainingFields::default(),
            source_order: 0,
        }];

        let suggestions = build_review_suggestions(
            &records,
            ReviewSettings {
                problem_interval_days: 21,
                tag_window_days: 60,
                tag_target_problems: 5,
            },
            Some(&HashSet::from(["二分".to_string()])),
        );

        assert_eq!(
            suggestions.problem_reviews,
            vec![ProblemReviewCandidate {
                problem_id: "luogu:P2001".to_string(),
                title: "Stable".to_string(),
                verdict: "AC".to_string(),
                last_submission_time: records[0].submission_time,
                priority: 2,
                reasons: vec!["已达到 21 天复习间隔（距上次练习 50 天）".to_string()],
                matched_tags: vec!["二分".to_string()],
            }]
        );
    }

    #[test]
    fn build_review_suggestions_skips_interval_review_without_submission_time() {
        let records = vec![SolveRecord {
            problem_id: "luogu:P3001".to_string(),
            provider: crate::problem::ProblemProvider::Luogu,
            title: "NoTime".to_string(),
            verdict: "AC".to_string(),
            score: None,
            time_ms: None,
            memory_mb: None,
            difficulty: "入门".to_string(),
            tags: vec!["图论".to_string()],
            source: "Luogu".to_string(),
            contest: None,
            submission_id: None,
            submission_time: None,
            file_name: "P3001.cpp".to_string(),
            training: TrainingFields::default(),
            source_order: 0,
        }];

        let suggestions = build_review_suggestions(
            &records,
            ReviewSettings {
                problem_interval_days: 21,
                tag_window_days: 60,
                tag_target_problems: 5,
            },
            Some(&HashSet::from(["图论".to_string()])),
        );

        assert!(suggestions.problem_reviews.is_empty());
    }

    #[test]
    fn build_review_suggestions_generates_tag_practice_without_weakness_label() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
        let records = vec![
            SolveRecord {
                problem_id: "luogu:P4001".to_string(),
                provider: crate::problem::ProblemProvider::Luogu,
                title: "A".to_string(),
                verdict: "AC".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["数论".to_string()],
                source: "Luogu".to_string(),
                contest: None,
                submission_id: Some(1),
                submission_time: Some(now - chrono::Duration::days(10)),
                file_name: "P4001.cpp".to_string(),
                training: TrainingFields::default(),
                source_order: 0,
            },
            SolveRecord {
                problem_id: "luogu:P4002".to_string(),
                provider: crate::problem::ProblemProvider::Luogu,
                title: "B".to_string(),
                verdict: "WA".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["数论".to_string()],
                source: "Luogu".to_string(),
                contest: None,
                submission_id: Some(2),
                submission_time: Some(now - chrono::Duration::days(5)),
                file_name: "P4002.cpp".to_string(),
                training: TrainingFields {
                    confidence: Some("low".to_string()),
                    ..TrainingFields::default()
                },
                source_order: 1,
            },
        ];

        let suggestions = build_review_suggestions(
            &records,
            ReviewSettings {
                problem_interval_days: 21,
                tag_window_days: 60,
                tag_target_problems: 5,
            },
            Some(&HashSet::from(["数论".to_string()])),
        );

        assert_eq!(
            suggestions.tag_practice_suggestions,
            vec![TagPracticeSuggestion {
                tag: "数论".to_string(),
                recent_unique_problems: 2,
                lifetime_unique_problems: 2,
                priority: 298,
                reason: "最近 60 天仅练过 2 题，建议补样本；最近这类题也有 1 次不稳信号"
                    .to_string(),
                recent_unstable_signal_count: 1,
            }]
        );
    }

    #[test]
    fn build_review_suggestions_sorts_problem_reviews_by_priority_then_time() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
        let records = vec![
            SolveRecord {
                problem_id: "luogu:P5001".to_string(),
                provider: crate::problem::ProblemProvider::Luogu,
                title: "Late".to_string(),
                verdict: "AC".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string()],
                source: "Luogu".to_string(),
                contest: None,
                submission_id: Some(1),
                submission_time: Some(now - chrono::Duration::days(70)),
                file_name: "P5001.cpp".to_string(),
                training: TrainingFields::default(),
                source_order: 0,
            },
            SolveRecord {
                problem_id: "luogu:P5002".to_string(),
                provider: crate::problem::ProblemProvider::Luogu,
                title: "Wrong".to_string(),
                verdict: "WA".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                tags: vec!["模拟".to_string()],
                source: "Luogu".to_string(),
                contest: None,
                submission_id: Some(2),
                submission_time: Some(now - chrono::Duration::days(2)),
                file_name: "P5002.cpp".to_string(),
                training: TrainingFields::default(),
                source_order: 1,
            },
        ];

        let suggestions = build_review_suggestions(
            &records,
            ReviewSettings {
                problem_interval_days: 21,
                tag_window_days: 60,
                tag_target_problems: 5,
            },
            Some(&HashSet::from(["模拟".to_string()])),
        );

        assert_eq!(
            suggestions
                .problem_reviews
                .iter()
                .map(|item| item.problem_id.as_str())
                .collect::<Vec<_>>(),
            vec!["luogu:P5002", "luogu:P5001"]
        );
    }

    #[test]
    fn latest_records_by_file_uses_latest_record_per_path() {
        let records = vec![
            HistoricalSolveRecord {
                revision: "old".to_string(),
                record: SolveRecord {
                    problem_id: "luogu:P1001".to_string(),
                    provider: crate::problem::ProblemProvider::Luogu,
                    title: "A".to_string(),
                    verdict: "WA".to_string(),
                    score: None,
                    time_ms: None,
                    memory_mb: None,
                    difficulty: "入门".to_string(),
                    tags: vec!["模拟".to_string()],
                    source: "Luogu".to_string(),
                    contest: None,
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
                    problem_id: "luogu:P1001".to_string(),
                    provider: crate::problem::ProblemProvider::Luogu,
                    title: "A".to_string(),
                    verdict: "AC".to_string(),
                    score: None,
                    time_ms: None,
                    memory_mb: None,
                    difficulty: "入门".to_string(),
                    tags: vec!["模拟".to_string()],
                    source: "Luogu".to_string(),
                    contest: None,
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
                problem_id: "luogu:P1001".to_string(),
                provider: crate::problem::ProblemProvider::Luogu,
                title: "A".to_string(),
                file_name: "solutions/P1001.cpp".to_string(),
                verdict: "AC".to_string(),
                score: None,
                time_ms: None,
                memory_mb: None,
                difficulty: "入门".to_string(),
                source: "Luogu".to_string(),
                contest: None,
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
