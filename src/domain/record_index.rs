use std::collections::BTreeMap;

use crate::domain::record::{
    FileRecordSummary, HistoricalSolveRecord, ProblemRecordSummary, SolveRecord,
};

#[derive(Debug, Clone, Default)]
pub struct RecordIndex {
    records: Vec<HistoricalSolveRecord>,
    current_by_file: Vec<FileRecordSummary>,
    current_by_problem: Vec<ProblemRecordSummary>,
    timelines_by_file: BTreeMap<String, Vec<HistoricalSolveRecord>>,
    timelines_by_problem: BTreeMap<String, Vec<HistoricalSolveRecord>>,
}

impl RecordIndex {
    pub fn build(records: &[HistoricalSolveRecord]) -> Self {
        let mut timelines_by_file: BTreeMap<String, Vec<HistoricalSolveRecord>> = BTreeMap::new();
        let mut timelines_by_problem: BTreeMap<String, Vec<HistoricalSolveRecord>> =
            BTreeMap::new();

        for entry in records {
            timelines_by_file
                .entry(entry.record.file_name.clone())
                .or_default()
                .push(entry.clone());
            timelines_by_problem
                .entry(entry.record.problem_id.clone())
                .or_default()
                .push(entry.clone());
        }

        for timeline in timelines_by_file.values_mut() {
            sort_timeline(timeline);
        }
        for timeline in timelines_by_problem.values_mut() {
            sort_timeline(timeline);
        }

        let current_by_file = timelines_by_file
            .values()
            .filter_map(|timeline| timeline.first())
            .map(to_file_summary)
            .collect::<Vec<_>>();

        let current_by_problem = timelines_by_problem
            .values()
            .filter_map(|timeline| timeline.first())
            .map(|entry| ProblemRecordSummary {
                problem_id: entry.record.problem_id.clone(),
                title: entry.record.title.clone(),
                verdict: entry.record.verdict.clone(),
                difficulty: entry.record.difficulty.clone(),
                tags: entry.record.tags.clone(),
                submission_id: entry.record.submission_id,
                submission_time: entry.record.submission_time,
                files: timelines_by_file
                    .iter()
                    .filter_map(|(file, items)| {
                        items.first().and_then(|item| {
                            if item.record.problem_id == entry.record.problem_id {
                                Some(file.clone())
                            } else {
                                None
                            }
                        })
                    })
                    .collect(),
                latest_revision: entry.revision.clone(),
            })
            .collect::<Vec<_>>();

        Self {
            records: records.to_vec(),
            current_by_file,
            current_by_problem,
            timelines_by_file,
            timelines_by_problem,
        }
    }

    pub fn all_records(&self) -> &[HistoricalSolveRecord] {
        &self.records
    }

    pub fn current_by_file(&self) -> &[FileRecordSummary] {
        &self.current_by_file
    }

    pub fn current_by_problem(&self) -> &[ProblemRecordSummary] {
        &self.current_by_problem
    }

    pub fn timeline_for_file(&self, file_name: &str) -> &[HistoricalSolveRecord] {
        self.timelines_by_file
            .get(file_name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn timeline_for_problem(&self, problem_id: &str) -> &[HistoricalSolveRecord] {
        self.timelines_by_problem
            .get(problem_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

pub fn latest_records_by_file(records: &[HistoricalSolveRecord]) -> Vec<FileRecordSummary> {
    RecordIndex::build(records).current_by_file().to_vec()
}

fn sort_timeline(records: &mut [HistoricalSolveRecord]) {
    records.sort_by(|left, right| compare_solve_records(&left.record, &right.record));
}

fn compare_solve_records(left: &SolveRecord, right: &SolveRecord) -> std::cmp::Ordering {
    match (left.submission_time, right.submission_time) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.source_order.cmp(&right.source_order),
    }
}

fn to_file_summary(entry: &HistoricalSolveRecord) -> FileRecordSummary {
    FileRecordSummary {
        revision: entry.revision.clone(),
        problem_id: entry.record.problem_id.clone(),
        title: entry.record.title.clone(),
        file_name: entry.record.file_name.clone(),
        verdict: entry.record.verdict.clone(),
        score: entry.record.score,
        time_ms: entry.record.time_ms,
        memory_mb: entry.record.memory_mb,
        difficulty: entry.record.difficulty.clone(),
        source: entry.record.source.clone(),
        tags: entry.record.tags.clone(),
        submission_id: entry.record.submission_id,
        submission_time: entry.record.submission_time,
        training: entry.record.training.clone(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};

    use super::RecordIndex;
    use crate::domain::record::{HistoricalSolveRecord, SolveRecord, TrainingFields};

    #[test]
    fn record_index_exposes_current_views_and_timelines() {
        let records = vec![
            HistoricalSolveRecord {
                revision: "old".to_string(),
                record: SolveRecord {
                    problem_id: "P1001".to_string(),
                    title: "A".to_string(),
                    verdict: "WA".to_string(),
                    score: Some(50),
                    time_ms: Some(10),
                    memory_mb: Some(1.2),
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
                    score: Some(100),
                    time_ms: Some(5),
                    memory_mb: Some(1.0),
                    difficulty: "入门".to_string(),
                    tags: vec!["模拟".to_string()],
                    source: "Luogu".to_string(),
                    submission_id: Some(2),
                    submission_time: Some(
                        FixedOffset::east_opt(8 * 3600)
                            .unwrap()
                            .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
                            .single()
                            .unwrap(),
                    ),
                    file_name: "P1001.cpp".to_string(),
                    training: TrainingFields::default(),
                    source_order: 0,
                },
            },
        ];

        let index = RecordIndex::build(&records);
        assert_eq!(index.current_by_file().len(), 1);
        assert_eq!(index.current_by_problem().len(), 1);
        assert_eq!(index.timeline_for_file("P1001.cpp").len(), 2);
        assert_eq!(index.timeline_for_problem("P1001").len(), 2);
        assert_eq!(index.current_by_file()[0].revision, "new");
    }
}
