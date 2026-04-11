use std::{collections::HashMap, fs, path::PathBuf};

use color_eyre::{Result, eyre::WrapErr};
use tracing::{info, instrument};

use crate::{
    config::AclogPaths,
    domain::{
        browser::{BrowserProviderView, BrowserQuery, BrowserRootView},
        record_index::RecordIndex,
        sync_batch::{SyncBatchSession, SyncItemStatus},
    },
    ui::interaction::{
        HomeAction, HomeLatestRecordSummary, HomeProviderSummary, HomeRecordListRow, HomeSummary,
        HomeSyncSessionSummary, UserInterface,
    },
};

use super::{deps::AppDeps, stats, support::load_record_index, sync};

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(workspace: PathBuf, deps: &impl AppDeps, ui: &impl UserInterface) -> Result<()> {
    info!("开始打开全局训练工作台");

    loop {
        let paths = AclogPaths::new(workspace.clone())?;
        deps.ensure_workspace().await?;
        let summary = build_home_summary(&paths, deps).await?;

        match ui.open_home(&paths.workspace_root, &summary)? {
            HomeAction::StartSync => {
                sync::run(
                    paths.workspace_root.clone(),
                    sync::SyncOptions::default(),
                    deps,
                    ui,
                )
                .await?;
            }
            HomeAction::ResumeSync => {
                sync::run(
                    paths.workspace_root.clone(),
                    sync::SyncOptions {
                        resume: true,
                        ..sync::SyncOptions::default()
                    },
                    deps,
                    ui,
                )
                .await?;
            }
            HomeAction::OpenStats => {
                stats::run(
                    paths.workspace_root.clone(),
                    &stats::StatsOptions::default(),
                    deps,
                    ui,
                )
                .await?;
            }
            HomeAction::OpenBrowserFiles => {
                super::browser::run(
                    paths.workspace_root.clone(),
                    BrowserQuery {
                        provider: BrowserProviderView::All,
                        root_view: BrowserRootView::Files,
                        ..BrowserQuery::default()
                    },
                    deps,
                    ui,
                )
                .await?;
            }
            HomeAction::OpenBrowserProblems => {
                super::browser::run(
                    paths.workspace_root.clone(),
                    BrowserQuery {
                        provider: BrowserProviderView::All,
                        root_view: BrowserRootView::Problems,
                        ..BrowserQuery::default()
                    },
                    deps,
                    ui,
                )
                .await?;
            }
            HomeAction::Exit => return Ok(()),
        }
    }
}

pub(crate) async fn build_home_summary(
    paths: &AclogPaths,
    deps: &impl AppDeps,
) -> Result<HomeSummary> {
    let index = load_record_index(deps).await?;
    let sync_session = load_sync_session_summary(paths)?;
    let tracked_records = tracked_file_summaries(&index, deps).await?;
    let latest_record = index
        .all_records()
        .first()
        .map(|record| HomeLatestRecordSummary {
            problem_id: record.record.problem_id.clone(),
            provider: record.record.provider,
            title: record.record.title.clone(),
            file_name: record.record.file_name.clone(),
            verdict: record.record.verdict.clone(),
            submission_time: record.record.submission_time,
        });

    Ok(HomeSummary {
        total_solve_records: index.all_records().len(),
        unique_problem_count: index.current_by_problem().len(),
        tracked_record_count: tracked_records.len(),
        provider_summaries: summarize_providers(&index),
        latest_record,
        sync_session,
        record_rows: tracked_records
            .into_iter()
            .map(|record| HomeRecordListRow {
                file_name: record.file_name,
                problem_id: record.problem_id,
                verdict: record.verdict,
                difficulty: record.difficulty,
                submission_id: record.submission_id,
                submission_time: record.submission_time,
                title: record.title,
            })
            .collect(),
    })
}

async fn tracked_file_summaries(
    index: &RecordIndex,
    deps: &impl AppDeps,
) -> Result<Vec<crate::domain::record::FileRecordSummary>> {
    let mut tracked = Vec::new();
    for summary in index.current_by_file() {
        if deps.is_tracked_file(&summary.file_name).await? {
            tracked.push(summary.clone());
        }
    }
    Ok(tracked)
}

fn summarize_providers(index: &RecordIndex) -> Vec<HomeProviderSummary> {
    let mut totals = HashMap::new();
    let mut uniques = HashMap::new();

    for record in index.all_records() {
        *totals.entry(record.record.provider).or_insert(0usize) += 1;
    }
    for problem in index.current_by_problem() {
        *uniques.entry(problem.provider).or_insert(0usize) += 1;
    }

    [
        crate::problem::ProblemProvider::Luogu,
        crate::problem::ProblemProvider::AtCoder,
        crate::problem::ProblemProvider::Unknown,
    ]
    .into_iter()
    .filter_map(|provider| {
        let total_solve_records = totals.get(&provider).copied().unwrap_or_default();
        let unique_problem_count = uniques.get(&provider).copied().unwrap_or_default();
        (total_solve_records > 0 || unique_problem_count > 0).then_some(HomeProviderSummary {
            provider,
            total_solve_records,
            unique_problem_count,
        })
    })
    .collect()
}

fn load_sync_session_summary(paths: &AclogPaths) -> Result<Option<HomeSyncSessionSummary>> {
    if !paths.sync_session_file.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&paths.sync_session_file).wrap_err_with(|| {
        format!(
            "读取 sync 会话文件 {} 失败",
            paths.sync_session_file.display()
        )
    })?;
    let session: SyncBatchSession = toml::from_str(&raw).wrap_err("sync 会话文件格式无效")?;
    let pending_items = session
        .items
        .iter()
        .filter(|item| item.status == SyncItemStatus::Pending)
        .count();
    let decided_items = session
        .items
        .iter()
        .filter(|item| item.is_decided())
        .count();
    Ok(Some(HomeSyncSessionSummary {
        created_at: session.created_at,
        total_items: session.items.len(),
        pending_items,
        decided_items,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{FixedOffset, TimeZone};

    use super::{load_sync_session_summary, summarize_providers};
    use crate::{
        config::AclogPaths,
        domain::{
            record_index::RecordIndex,
            sync_batch::{SyncBatchSession, SyncChangeKind, SyncItemStatus, SyncSessionItem},
        },
        ui::interaction::HomeSyncSessionSummary,
    };

    #[test]
    fn summarize_providers_counts_records_and_unique_problems() {
        let index = RecordIndex::build(&crate::commit_format::parse_historical_solve_records(&[
            (
                "rev-l".to_string(),
                "solve(P1001): A\n\nVerdict: AC\nDifficulty: 入门\nSource: Luogu\nSubmission-ID: 1\nSubmission-Time: 2024-01-01T00:00:00+08:00\nFile: P1001.cpp"
                    .to_string(),
            ),
            (
                "rev-a".to_string(),
                "solve(atcoder:abc350_a): B\n\nVerdict: WA\nDifficulty: C\nSource: AtCoder\nContest: ABC350\nSubmission-ID: 2\nSubmission-Time: 2024-01-02T00:00:00+08:00\nFile: abc350_a.cpp"
                    .to_string(),
            ),
        ]));

        let summary = summarize_providers(&index);
        assert_eq!(summary.len(), 2);
        assert_eq!(summary[0].total_solve_records, 1);
        assert_eq!(summary[1].unique_problem_count, 1);
    }

    #[test]
    fn load_sync_session_summary_reports_pending_and_decided_counts() {
        let workspace = tempfile::tempdir().unwrap();
        let aclog_dir = workspace.path().join(".aclog");
        fs::create_dir_all(&aclog_dir).unwrap();
        let paths = AclogPaths::new(workspace.path().to_path_buf()).unwrap();
        let session = SyncBatchSession {
            created_at: FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 3, 0, 0, 0)
                .single()
                .unwrap(),
            items: vec![
                sample_sync_item(SyncItemStatus::Pending),
                sample_sync_item(SyncItemStatus::Planned),
            ],
        };
        fs::write(
            &paths.sync_session_file,
            format!("{}\n", toml::to_string_pretty(&session).unwrap()),
        )
        .unwrap();

        assert_eq!(
            load_sync_session_summary(&paths).unwrap(),
            Some(HomeSyncSessionSummary {
                created_at: session.created_at,
                total_items: 2,
                pending_items: 1,
                decided_items: 1,
            })
        );
    }

    fn sample_sync_item(status: SyncItemStatus) -> SyncSessionItem {
        SyncSessionItem {
            file: "tracked/P1001.cpp".to_string(),
            problem_id: Some("luogu:P1001".to_string()),
            provider: crate::problem::ProblemProvider::Luogu,
            contest: None,
            kind: SyncChangeKind::Active,
            status,
            submissions: Some(1),
            default_submission_id: Some(1),
            decision: None,
            warnings: Vec::new(),
            invalid_reason: None,
        }
    }
}
