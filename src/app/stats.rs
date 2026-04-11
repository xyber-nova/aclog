use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::{
    config::AclogPaths,
    domain::browser::BrowserProviderView,
    domain::stats::{
        ReviewSettings, StatsDashboard, StatsProviderDashboard, build_review_suggestions,
        summarize_solve_records_with_window,
    },
    problem::ProblemProvider,
    ui::interaction::UserInterface,
};

use super::{deps::AppDeps, support::load_record_index};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatsOptions {
    pub days: Option<i64>,
    pub review: bool,
    pub json: bool,
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(
    workspace: PathBuf,
    options: &StatsOptions,
    deps: &impl AppDeps,
    ui: &impl UserInterface,
) -> Result<()> {
    info!("开始统计");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_workspace().await?;
    let config = crate::config::load_config(&paths)?;
    let index = load_record_index(deps).await?;
    let records = index
        .all_records()
        .iter()
        .map(|entry| entry.record.clone())
        .collect::<Vec<_>>();
    let luogu_algorithm_tag_names = deps
        .load_algorithm_tag_names(&config, &paths, ProblemProvider::Luogu)
        .await?;
    let review_settings = ReviewSettings {
        problem_interval_days: config.settings.review_problem_interval_days(),
        tag_window_days: config.settings.practice_tag_window_days(),
        tag_target_problems: config.settings.practice_tag_target_problems(),
    };
    let provider_dashboards = build_provider_dashboards(
        &records,
        options.days,
        review_settings,
        &luogu_algorithm_tag_names,
    );
    let default_dashboard = provider_dashboards
        .iter()
        .find(|item| item.provider == BrowserProviderView::Luogu)
        .or_else(|| {
            provider_dashboards
                .iter()
                .find(|item| item.provider == BrowserProviderView::All)
        })
        .cloned()
        .unwrap_or_else(|| StatsProviderDashboard {
            provider: BrowserProviderView::Luogu,
            summary: summarize_solve_records_with_window(
                &records,
                options.days,
                Some(&luogu_algorithm_tag_names),
            ),
            problem_reviews: Vec::new(),
            tag_practice_suggestions: Vec::new(),
            tag_features_supported: false,
        });

    if options.review {
        let output = if options.json {
            format!(
                "{}\n",
                serde_json::to_string_pretty(&crate::domain::stats::ReviewSuggestions {
                    problem_reviews: default_dashboard.problem_reviews.clone(),
                    tag_practice_suggestions: default_dashboard.tag_practice_suggestions.clone(),
                })?
            )
        } else {
            let dashboard = StatsDashboard {
                summary: default_dashboard.summary.clone(),
                problem_reviews: default_dashboard.problem_reviews.clone(),
                tag_practice_suggestions: default_dashboard.tag_practice_suggestions.clone(),
                start_in_review: true,
                provider_dashboards: provider_dashboards.clone(),
            };
            ui.show_stats_dashboard(&paths.workspace_root, &dashboard, &index)?;
            info!(
                problem_reviews = dashboard.problem_reviews.len(),
                tag_practice_suggestions = dashboard.tag_practice_suggestions.len(),
                "复习建议已输出"
            );
            return Ok(());
        };
        deps.write_output(&output)?;
        info!(
            problem_reviews = default_dashboard.problem_reviews.len(),
            tag_practice_suggestions = default_dashboard.tag_practice_suggestions.len(),
            "复习建议已输出"
        );
        return Ok(());
    }

    if options.json {
        deps.write_output(&format!(
            "{}\n",
            serde_json::to_string_pretty(&default_dashboard.summary)?
        ))?;
    } else {
        ui.show_stats_dashboard(
            &paths.workspace_root,
            &StatsDashboard {
                summary: default_dashboard.summary.clone(),
                problem_reviews: default_dashboard.problem_reviews.clone(),
                tag_practice_suggestions: default_dashboard.tag_practice_suggestions.clone(),
                start_in_review: false,
                provider_dashboards: provider_dashboards.clone(),
            },
            &index,
        )?;
    }

    info!(
        total_solve_records = default_dashboard.summary.total_solve_records,
        unique_problem_count = default_dashboard.summary.unique_problem_count,
        "统计完成"
    );
    Ok(())
}

fn build_provider_dashboards(
    records: &[crate::domain::record::SolveRecord],
    days: Option<i64>,
    settings: ReviewSettings,
    luogu_algorithm_tag_names: &std::collections::HashSet<String>,
) -> Vec<StatsProviderDashboard> {
    [
        BrowserProviderView::Luogu,
        BrowserProviderView::AtCoder,
        BrowserProviderView::All,
    ]
    .into_iter()
    .map(|provider| {
        let filtered_records = filter_records_by_provider(records, provider);
        let tag_features_supported = matches!(provider, BrowserProviderView::Luogu);
        let mut summary = summarize_solve_records_with_window(
            &filtered_records,
            days,
            tag_features_supported.then_some(luogu_algorithm_tag_names),
        );
        let mut suggestions = build_review_suggestions(
            &filtered_records,
            settings,
            tag_features_supported.then_some(luogu_algorithm_tag_names),
        );
        if !tag_features_supported {
            summary.tag_counts.clear();
            suggestions.tag_practice_suggestions.clear();
            for item in &mut suggestions.problem_reviews {
                item.matched_tags.clear();
            }
        }
        StatsProviderDashboard {
            provider,
            summary,
            problem_reviews: suggestions.problem_reviews,
            tag_practice_suggestions: suggestions.tag_practice_suggestions,
            tag_features_supported,
        }
    })
    .collect()
}

fn filter_records_by_provider(
    records: &[crate::domain::record::SolveRecord],
    provider: BrowserProviderView,
) -> Vec<crate::domain::record::SolveRecord> {
    records
        .iter()
        .filter(|record| match provider {
            BrowserProviderView::All => true,
            BrowserProviderView::Luogu => record.provider == ProblemProvider::Luogu,
            BrowserProviderView::AtCoder => record.provider == ProblemProvider::AtCoder,
        })
        .cloned()
        .collect()
}

#[cfg(test)]
fn render_review_suggestions(suggestions: &crate::domain::stats::ReviewSuggestions) -> String {
    if suggestions.problem_reviews.is_empty() && suggestions.tag_practice_suggestions.is_empty() {
        return "当前没有可用的复习建议或加练建议\n".to_string();
    }

    let mut lines = vec!["题目复习".to_string()];
    for item in &suggestions.problem_reviews {
        lines.push(format!(
            "{}\t{}\tP{}\t{}",
            item.problem_id,
            item.verdict,
            item.priority,
            item.reasons.join("；")
        ));
    }
    lines.push(String::new());
    lines.push("标签加练".to_string());
    for item in &suggestions.tag_practice_suggestions {
        lines.push(format!(
            "{}\t{}\t{}\t{}",
            item.tag, item.recent_unique_problems, item.priority, item.reason
        ));
    }
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use crate::domain::stats::{ProblemReviewCandidate, ReviewSuggestions, TagPracticeSuggestion};

    use super::render_review_suggestions;

    #[test]
    fn render_review_suggestions_outputs_two_sections() {
        let text = render_review_suggestions(&ReviewSuggestions {
            problem_reviews: vec![ProblemReviewCandidate {
                problem_id: "P1001".to_string(),
                title: "A".to_string(),
                verdict: "WA".to_string(),
                last_submission_time: None,
                priority: 6,
                reasons: vec!["最近状态仍为 WA".to_string()],
                matched_tags: vec!["模拟".to_string()],
            }],
            tag_practice_suggestions: vec![TagPracticeSuggestion {
                tag: "二分".to_string(),
                recent_unique_problems: 1,
                lifetime_unique_problems: 1,
                priority: 399,
                reason: "最近 60 天仅练过 1 题，建议补样本".to_string(),
                recent_unstable_signal_count: 0,
            }],
        });

        assert!(text.contains("题目复习"));
        assert!(text.contains("标签加练"));
        assert!(text.contains("建议补样本"));
    }
}
