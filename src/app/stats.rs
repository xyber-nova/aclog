use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::{
    config::AclogPaths,
    domain::stats::{
        ReviewSettings, StatsDashboard, build_review_suggestions,
        summarize_solve_records_with_window,
    },
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
    let algorithm_tag_names = deps.load_algorithm_tag_names(&config, &paths).await?;
    let suggestions = build_review_suggestions(
        &records,
        ReviewSettings {
            problem_interval_days: config.settings.review_problem_interval_days(),
            tag_window_days: config.settings.practice_tag_window_days(),
            tag_target_problems: config.settings.practice_tag_target_problems(),
        },
        Some(&algorithm_tag_names),
    );
    let summary =
        summarize_solve_records_with_window(&records, options.days, Some(&algorithm_tag_names));

    if options.review {
        let output = if options.json {
            format!("{}\n", serde_json::to_string_pretty(&suggestions)?)
        } else {
            let dashboard = StatsDashboard {
                summary,
                problem_reviews: suggestions.problem_reviews.clone(),
                tag_practice_suggestions: suggestions.tag_practice_suggestions.clone(),
                start_in_review: true,
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
            problem_reviews = suggestions.problem_reviews.len(),
            tag_practice_suggestions = suggestions.tag_practice_suggestions.len(),
            "复习建议已输出"
        );
        return Ok(());
    }

    if options.json {
        deps.write_output(&format!("{}\n", serde_json::to_string_pretty(&summary)?))?;
    } else {
        ui.show_stats_dashboard(
            &paths.workspace_root,
            &StatsDashboard {
                summary: summary.clone(),
                problem_reviews: suggestions.problem_reviews,
                tag_practice_suggestions: suggestions.tag_practice_suggestions,
                start_in_review: false,
            },
            &index,
        )?;
    }

    info!(
        total_solve_records = summary.total_solve_records,
        unique_problem_count = summary.unique_problem_count,
        "统计完成"
    );
    Ok(())
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
